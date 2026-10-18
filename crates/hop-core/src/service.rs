use crate::{
    config::{self, Config, Identity},
    crypto::{SecureConnection, SecureReader, SecureWriter},
    model::*,
    platform::Platform,
    topology,
};
use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot, watch},
};

pub(crate) struct State {
    pub local: Device,
    pub peers: PeerMap,
    discovered: BTreeMap<String, Vec<String>>,
    pub transfers: BTreeMap<String, TransferStatus>,
    pub diagnostics: Vec<String>,
    active: Option<Active>,
    inbound: Option<String>,
    away: Option<(String, String)>,
    layout_offers: BTreeMap<String, Vec<LayoutDisplay>>,
    shared_keys: BTreeMap<String, BTreeSet<u16>>,
    pressed: BTreeSet<u16>,
    consumed: BTreeSet<u16>,
    shortcut: topology::WindowsMacShortcut,
    local_position: (f64, f64),
    edge: Option<(String, Instant)>,
    buttons: BTreeSet<u8>,
    last_motion: (f64, f64),
    pub clipboard_hash: Vec<u8>,
}
struct RoutingConfig {
    settings: Settings,
    layout: Vec<LayoutDisplay>,
    peers: BTreeMap<String, bool>,
    hop_keys: Vec<HopKey>,
}
impl From<&Config> for RoutingConfig {
    fn from(c: &Config) -> Self {
        Self {
            settings: c.settings.clone(),
            layout: c.layout.clone(),
            peers: c
                .peers
                .iter()
                .map(|(id, p)| (id.clone(), p.allow_input))
                .collect(),
            hop_keys: c.hop_keys.clone(),
        }
    }
}
#[derive(Clone)]
struct Active {
    peer: String,
    display: String,
    x: f64,
    y: f64,
    return_x: f64,
    return_y: f64,
}
#[derive(Clone)]
struct Channel {
    generation: String,
    preferred: bool,
    tx: Arc<crate::outbound::Outbound>,
    priority: mpsc::UnboundedSender<Message>,
    epoch: Arc<AtomicU64>,
}
/// Disable multicast for embedded/manual-only engines and isolated TCP tests.
#[derive(Clone, Copy)]
pub struct StartOptions {
    pub discovery: bool,
    pub protect_identity: bool,
    /// Keep isolated protocol tests off LAN interfaces and firewall prompts.
    pub loopback_only: bool,
}
impl Default for StartOptions {
    fn default() -> Self {
        Self {
            discovery: true,
            protect_identity: true,
            loopback_only: false,
        }
    }
}
pub struct Service {
    pub(crate) identity: Identity,
    pub(crate) config: Mutex<Config>,
    routing: ArcSwap<RoutingConfig>,
    pub(crate) config_dir: PathBuf,
    pub(crate) state: Mutex<State>,
    pub(crate) backend: Arc<dyn Platform>,
    channels: Mutex<BTreeMap<String, Channel>>,
    pub(crate) decisions: Mutex<BTreeMap<String, oneshot::Sender<bool>>>,
    pub(crate) cancelled: Mutex<BTreeSet<String>>,
    pub(crate) stop: watch::Sender<bool>,
    pub(crate) outgoing_bulk: tokio::sync::Semaphore,
    incoming_bulk: tokio::sync::Semaphore,
    pub(crate) file_clipboard_hash: Mutex<Vec<u8>>,
    pub(crate) file_batches: Mutex<BTreeMap<String, crate::transfer::FileBatch>>,
    pub(crate) incoming_transfers: Mutex<BTreeSet<String>>,
    clipboard_retry: AtomicBool,
    pub(crate) session_available: AtomicBool,
    input_available: AtomicBool,
    capture_started: AtomicBool,
    keyboard_captured: AtomicU64,
    keyboard_forwarded: AtomicU64,
    keyboard_received: AtomicU64,
    keyboard_applied: AtomicU64,
    keyboard_rejected: AtomicU64,
    scroll_captured: AtomicU64,
    scroll_forwarded: AtomicU64,
    scroll_received: AtomicU64,
    scroll_applied: AtomicU64,
    scroll_rejected: AtomicU64,
    displays_available: AtomicBool,
    session_clock: Mutex<SystemTime>,
    metadata_changed: watch::Sender<u64>,
    pub listening_port: u16,
}
impl Service {
    pub async fn start(
        dir: PathBuf,
        port_override: Option<u16>,
        backend: Arc<dyn Platform>,
    ) -> Result<Arc<Self>> {
        Self::start_with_options(dir, port_override, backend, StartOptions::default()).await
    }
    pub async fn start_with_options(
        dir: PathBuf,
        port_override: Option<u16>,
        backend: Arc<dyn Platform>,
        options: StartOptions,
    ) -> Result<Arc<Self>> {
        let mut config = if options.protect_identity {
            config::load_native(&dir)?
        } else {
            config::load(&dir)?
        };
        let identity = Identity::from_config(&config)?;
        let listener = TcpListener::bind((
            if options.loopback_only {
                "127.0.0.1"
            } else {
                "0.0.0.0"
            },
            port_override.unwrap_or(config.settings.port),
        ))
        .await
        .context("Could not listen: port already in use or firewall denied access")?;
        let listening_port = listener.local_addr()?.port();
        let displays = backend.displays()?;
        anyhow::ensure!(!displays.is_empty(), "No displays found");
        let local = Device {
            id: identity.id.clone(),
            name: config.settings.device_name.clone(),
            platform: backend.name().into(),
            displays,
        };
        migrate_display_ids(&mut config, &local.id, &backend.display_id_aliases()?);
        topology::reconcile_device(&mut config.layout, &local);
        config::save(&dir, &config)?;
        let (stop, _) = watch::channel(false);
        let (metadata_changed, _) = watch::channel(0);
        let primary = local
            .displays
            .iter()
            .find(|d| d.primary)
            .unwrap_or(&local.displays[0]);
        let initial_position = (
            primary.bounds.x + primary.bounds.width / 2.,
            primary.bounds.y + primary.bounds.height / 2.,
        );
        let peers = config
            .peers
            .iter()
            .map(|(id, peer)| {
                let safety_code = hex::decode(&peer.public_key)
                    .ok()
                    .filter(|key| key.len() == 32 && config::device_id(key) == *id)
                    .map(|key| config::safety_code(&identity.public, &key));
                (
                    id.clone(),
                    PeerStatus {
                        device: Device {
                            id: id.clone(),
                            name: peer.name.clone(),
                            platform: "unknown".into(),
                            displays: vec![],
                        },
                        address: peer.address.clone().unwrap_or_default(),
                        public_key: Some(peer.public_key.clone()),
                        safety_code,
                        trusted: true,
                        connected: false,
                        latency_ms: None,
                        last_error: None,
                    },
                )
            })
            .collect();
        let session_available = backend.session_available();
        let input_available = backend.input_available();
        let this = Arc::new(Self {
            identity,
            routing: ArcSwap::from_pointee(RoutingConfig::from(&config)),
            config: Mutex::new(config),
            config_dir: dir,
            state: Mutex::new(State {
                local,
                peers,
                discovered: BTreeMap::new(),
                transfers: BTreeMap::new(),
                diagnostics: vec![],
                active: None,
                inbound: None,
                away: None,
                layout_offers: BTreeMap::new(),
                shared_keys: BTreeMap::new(),
                pressed: BTreeSet::new(),
                consumed: BTreeSet::new(),
                shortcut: topology::WindowsMacShortcut::default(),
                local_position: initial_position,
                edge: None,
                buttons: BTreeSet::new(),
                last_motion: (0., 0.),
                clipboard_hash: vec![],
            }),
            backend,
            channels: Mutex::new(BTreeMap::new()),
            decisions: Mutex::new(BTreeMap::new()),
            cancelled: Mutex::new(BTreeSet::new()),
            stop,
            outgoing_bulk: tokio::sync::Semaphore::new(4),
            incoming_bulk: tokio::sync::Semaphore::new(4),
            incoming_transfers: Mutex::new(BTreeSet::new()),
            clipboard_retry: AtomicBool::new(false),
            file_clipboard_hash: Mutex::new(Vec::new()),
            file_batches: Mutex::new(BTreeMap::new()),
            session_available: AtomicBool::new(session_available),
            input_available: AtomicBool::new(input_available),
            capture_started: AtomicBool::new(false),
            keyboard_captured: AtomicU64::new(0),
            keyboard_forwarded: AtomicU64::new(0),
            keyboard_received: AtomicU64::new(0),
            keyboard_applied: AtomicU64::new(0),
            keyboard_rejected: AtomicU64::new(0),
            scroll_captured: AtomicU64::new(0),
            scroll_forwarded: AtomicU64::new(0),
            scroll_received: AtomicU64::new(0),
            scroll_applied: AtomicU64::new(0),
            scroll_rejected: AtomicU64::new(0),
            displays_available: AtomicBool::new(true),
            session_clock: Mutex::new(SystemTime::now()),
            listening_port,
            metadata_changed,
        });
        let weak = Arc::downgrade(&this);
        if let Err(e) = this.backend.start_capture(Arc::new(move |event| {
            weak.upgrade().is_some_and(|s| s.capture(event))
        })) {
            this.input_available.store(false, Ordering::SeqCst);
            this.note(format!("Input capture: {e}"));
        } else {
            this.capture_started.store(true, Ordering::SeqCst);
        }
        this.note(format!(
            "Listening on TCP {listening_port}; encrypted connections only"
        ));
        let s = this.clone();
        tokio::spawn(async move {
            let mut stop = s.stop.subscribe();
            if *stop.borrow() {
                return;
            }
            let permits = Arc::new(tokio::sync::Semaphore::new(32));
            loop {
                tokio::select! {_ = stop.changed()=>break,result=listener.accept()=>match result{Ok((stream,address))=>{
                    if !local_ip(address.ip()){continue;}let Ok(permit)=permits.clone().try_acquire_owned() else {continue;};
                    let s=s.clone();tokio::spawn(async move{
                        let mut stop = s.stop.subscribe(); if *stop.borrow() { return; }
                        tokio::select! { biased; _=stop.changed()=>{}, result=s.accept(stream,address,permit)=>{if let Err(e)=result{s.note(format!("Incoming connection: {e}"));}} }
                    });
                },Err(e)=>s.note(format!("Listener: {e}"))}}
            }
        });
        if options.discovery {
            this.start_discovery();
        }
        this.start_reconnect();
        this.start_clipboard();
        this.start_display_refresh();
        this.start_session_monitor();
        Ok(this)
    }
    /// Called only by an explicit user action; never prompts during background work.
    pub fn request_input_permissions(self: &Arc<Self>) -> Result<Permissions> {
        let permissions = self.backend.request_permissions();
        if permissions.capture && permissions.injection {
            self.release();
            self.backend.stop_capture();
            self.capture_started.store(false, Ordering::SeqCst);
            self.input_available.store(false, Ordering::SeqCst);
            let weak = Arc::downgrade(self);
            self.backend.start_capture(Arc::new(move |event| {
                weak.upgrade().is_some_and(|s| s.capture(event))
            }))?;
            self.capture_started.store(true, Ordering::SeqCst);
            self.input_available.store(
                self.backend.input_available() && self.displays_available.load(Ordering::SeqCst),
                Ordering::SeqCst,
            );
        }
        Ok(permissions)
    }
    pub fn snapshot(&self) -> Snapshot {
        let c = self.config.lock().unwrap();
        let s = self.state.lock().unwrap();
        Snapshot {
            local: s.local.clone(),
            public_key: hex::encode(&self.identity.public),
            fingerprint: config::fingerprint(&self.identity.public),
            settings: c.settings.clone(),
            peers: s.peers.values().cloned().collect(),
            layout: c.layout.clone(),
            layout_offers: s
                .layout_offers
                .iter()
                .map(|(id, layout)| LayoutOffer {
                    peer_id: id.clone(),
                    layout: layout.clone(),
                })
                .collect(),
            profiles: c.profiles.keys().cloned().collect(),
            hop_keys: c.hop_keys.clone(),
            peer_permissions: c
                .peers
                .iter()
                .map(|(id, p)| {
                    (
                        id.clone(),
                        PeerPermissions {
                            input: p.allow_input,
                            clipboard: p.allow_clipboard,
                            files: p.allow_files,
                        },
                    )
                })
                .collect(),
            transfers: s.transfers.values().cloned().collect(),
            permissions: self.backend.permissions(),
            capture_ready: self.capture_started.load(Ordering::SeqCst),
            keyboard_diagnostics: KeyboardDiagnostics {
                captured: self.keyboard_captured.load(Ordering::Relaxed),
                forwarded: self.keyboard_forwarded.load(Ordering::Relaxed),
                received: self.keyboard_received.load(Ordering::Relaxed),
                applied: self.keyboard_applied.load(Ordering::Relaxed),
                rejected: self.keyboard_rejected.load(Ordering::Relaxed),
                native: self.backend.keyboard_capture_stats(),
            },
            scroll_diagnostics: ScrollDiagnostics {
                captured: self.scroll_captured.load(Ordering::Relaxed),
                forwarded: self.scroll_forwarded.load(Ordering::Relaxed),
                received: self.scroll_received.load(Ordering::Relaxed),
                applied: self.scroll_applied.load(Ordering::Relaxed),
                rejected: self.scroll_rejected.load(Ordering::Relaxed),
                natural_scrolling: self.backend.natural_scrolling(),
            },
            control_queues: self.channels.lock().unwrap().values().fold(
                ControlQueueDiagnostics::default(),
                |mut sum, channel| {
                    let stats = channel.tx.diagnostics();
                    sum.pending += stats.pending;
                    sum.peak_pending = sum.peak_pending.max(stats.peak_pending);
                    sum.coalesced_moves += stats.coalesced_moves;
                    sum.full_rejections += stats.full_rejections;
                    sum.longest_wait_us = sum.longest_wait_us.max(stats.longest_wait_us);
                    sum
                },
            ),
            displays_ready: self.displays_available.load(Ordering::SeqCst),
            active_device: s
                .active
                .as_ref()
                .map(|a| a.peer.clone())
                .or_else(|| s.away.as_ref().map(|(_, target)| target.clone()))
                .unwrap_or_else(|| s.local.id.clone()),
            controller_device: if s.active.is_some() {
                s.local.id.clone()
            } else {
                s.inbound
                    .clone()
                    .or_else(|| s.away.as_ref().map(|(owner, _)| owner.clone()))
                    .unwrap_or_else(|| s.local.id.clone())
            },
            cursor_visible: s.active.is_none() && s.away.is_none(),
            listening_port: self.listening_port,
            diagnostics: s.diagnostics.clone(),
            version: env!("CARGO_PKG_VERSION").into(),
            suppressed_events: self.backend.suppressed_events(),
            session_available: self.session_available.load(Ordering::SeqCst),
        }
    }
    pub(crate) fn note(&self, text: String) {
        tracing::info!("{text}");
        let mut s = self.state.lock().unwrap();
        if s.diagnostics.last() != Some(&text) {
            s.diagnostics.push(text);
            if s.diagnostics.len() > 100 {
                s.diagnostics.remove(0);
            }
        }
    }
    pub fn settings(&self, settings: Settings) -> Result<()> {
        settings.validate()?;
        anyhow::ensure!(
            Path::new(&settings.receive_directory).is_absolute(),
            "Receiving folder must be an absolute path"
        );
        let previous = self.routing.load();
        let should_release = settings.paused
            || settings.map_modifiers != previous.settings.map_modifiers
            || settings.mac_alt_tab != previous.settings.mac_alt_tab
            || settings.mac_alt_tab_peers != previous.settings.mac_alt_tab_peers;
        if previous.settings.clipboard_files != settings.clipboard_files {
            self.file_clipboard_hash.lock().unwrap().clear();
        }
        let renamed = previous.settings.device_name != settings.device_name;
        let mut c = self.config.lock().unwrap();
        let mut next = c.clone();
        next.settings = settings;
        config::save(&self.config_dir, &next)?;
        *c = next;
        self.update_routing(&c);
        self.state.lock().unwrap().local.name = c.settings.device_name.clone();
        drop(c);
        if renamed {
            self.disconnect_controls();
            self.metadata_changed.send_modify(|n| *n += 1);
        }
        if should_release {
            self.release();
            self.cancel_peer_files(None);
        }
        Ok(())
    }
    pub fn set_layout(&self, layout: Vec<LayoutDisplay>) -> Result<()> {
        topology::validate(&layout)?;
        let existing = self.config.lock().unwrap().layout.clone();
        let s = self.state.lock().unwrap();
        for l in &layout {
            let device = if l.device_id == s.local.id {
                Some(&s.local)
            } else {
                s.peers.get(&l.device_id).map(|p| &p.device)
            };
            // Persisted offline displays are allowed only if already known.
            let known = existing
                .iter()
                .any(|old| old.device_id == l.device_id && old.display_id == l.display_id);
            anyhow::ensure!(
                known || device.is_some_and(|d| d.displays.iter().any(|d| d.id == l.display_id)),
                "Unknown display"
            );
        }
        drop(s);
        let mut c = self.config.lock().unwrap();
        let mut next = c.clone();
        next.layout = layout;
        config::save(&self.config_dir, &next)?;
        *c = next;
        self.update_routing(&c);
        drop(c);
        self.release();
        Ok(())
    }
    pub fn approve_peer(&self, id: &str, expected_key: &str) -> Result<()> {
        let pending = {
            let s = self.state.lock().unwrap();
            s.peers
                .get(id)
                .cloned()
                .context("Connect first to verify this computer's identity")?
        };
        anyhow::ensure!(
            pending.public_key.as_deref() == Some(expected_key),
            "Peer identity changed. Compare fingerprints again"
        );
        let key = hex::decode(expected_key)?;
        anyhow::ensure!(
            key.len() == 32 && config::device_id(&key) == id,
            "Invalid peer identity"
        );
        let mut c = self.config.lock().unwrap();
        let mut next = c.clone();
        next.peers.insert(
            id.into(),
            TrustedPeer {
                public_key: expected_key.into(),
                name: pending.device.name.clone(),
                address: Some(pending.address.clone()),
                allow_input: true,
                allow_clipboard: true,
                allow_files: true,
            },
        );
        config::save(&self.config_dir, &next)?;
        *c = next;
        self.update_routing(&c);
        drop(c);
        self.state
            .lock()
            .unwrap()
            .peers
            .get_mut(id)
            .unwrap()
            .trusted = true;
        self.note(format!("Approved computer {}", pending.device.name));
        Ok(())
    }
    pub fn set_hop_key(&self, mut binding: HopKey) -> Result<()> {
        binding.keys.sort_unstable();
        binding.keys.dedup();
        anyhow::ensure!(
            binding.keys.is_empty()
                || (binding.keys.len() >= 2
                    && binding.keys.len() <= 4
                    && binding.keys.iter().all(|k| (4..=231).contains(k))
                    && binding.keys.iter().any(|k| *k >= 224)),
            "Use a shortcut with 2–4 keys including a modifier"
        );
        anyhow::ensure!(
            !binding.keys.contains(&41),
            "Escape is reserved for emergency recovery"
        );
        let mut c = self.config.lock().unwrap();
        anyhow::ensure!(
            c.layout
                .iter()
                .any(|d| d.device_id == binding.device_id && d.display_id == binding.display_id),
            "Unknown display"
        );
        anyhow::ensure!(
            binding.keys.is_empty()
                || !c.hop_keys.iter().any(|b| b.keys == binding.keys
                    && (b.device_id != binding.device_id || b.display_id != binding.display_id)),
            "This shortcut is already assigned to another display"
        );
        let mut next = c.clone();
        next.hop_keys
            .retain(|b| b.device_id != binding.device_id || b.display_id != binding.display_id);
        if !binding.keys.is_empty() {
            next.hop_keys.push(binding);
        }
        config::save(&self.config_dir, &next)?;
        *c = next;
        self.update_routing(&c);
        Ok(())
    }
    pub fn jump_to_display(&self, device_id: &str, display_id: &str) -> Result<()> {
        anyhow::ensure!(
            self.session_available.load(Ordering::SeqCst)
                && self.input_available.load(Ordering::SeqCst),
            "Input sharing is unavailable in the current OS session"
        );
        let c = self.routing.load();
        anyhow::ensure!(
            !c.settings.paused,
            "Resume sharing before jumping to a display"
        );
        let placement = c
            .layout
            .iter()
            .find(|d| d.device_id == device_id && d.display_id == display_id && !d.hidden)
            .context("Display is hidden or unavailable")?
            .clone();
        let (local, native, return_position) = {
            let s = self.state.lock().unwrap();
            let native = if device_id == s.local.id {
                s.local.displays.iter().find(|d| d.id == display_id)
            } else {
                anyhow::ensure!(
                    s.peers.get(device_id).is_some_and(|p| p.connected)
                        && c.peers.get(device_id).is_some_and(|p| *p),
                    "Computer is offline or input sharing is disabled"
                );
                s.peers
                    .get(device_id)
                    .and_then(|p| p.device.displays.iter().find(|d| d.id == display_id))
            };
            (
                s.local.clone(),
                native.context("Physical display unavailable")?.clone(),
                s.local_position,
            )
        };
        self.release();
        let x = placement.rect.x + placement.rect.width / 2.;
        let y = placement.rect.y + placement.rect.height / 2.;
        let (nx, ny) = topology::to_native(&placement.rect, &native.bounds, x, y);
        if device_id == local.id {
            self.backend.inject(&InputEvent::Move { x: nx, y: ny })?;
            self.restore_physical();
            return Ok(());
        }
        if !self.backend.manages_cursor_parking() {
            let park = local
                .displays
                .iter()
                .find(|d| d.bounds.contains(return_position.0, return_position.1))
                .unwrap_or(&local.displays[0]);
            self.backend.inject(&InputEvent::Move {
                x: park.bounds.x + park.bounds.width / 2.,
                y: park.bounds.y + park.bounds.height / 2.,
            })?;
        }
        self.release_physical_on_source();
        let mut state = self.state.lock().unwrap();
        // Publish the Enter barrier while ownership is locked. A keyboard
        // callback on another native thread must never enqueue its first key
        // ahead of the remote lease.
        let entered = self.queue_enter_locked(&state, device_id, display_id, nx, ny, true);
        if !entered {
            drop(state);
            self.send(device_id, Message::Leave);
            self.release();
            anyhow::bail!("Input queue unavailable; returned input locally");
        }
        state.shortcut.reset();
        state.active = Some(Active {
            peer: device_id.into(),
            display: display_id.into(),
            x,
            y,
            return_x: return_position.0,
            return_y: return_position.1,
        });
        drop(state);
        if let Err(e) = self.sync_forwarding() {
            self.release();
            return Err(e.context("Cannot suppress local input"));
        }
        self.announce_focus(device_id);
        Ok(())
    }
    pub fn save_profile(&self, name: &str) -> Result<()> {
        anyhow::ensure!(
            !name.trim().is_empty()
                && name.chars().count() <= 48
                && !name.chars().any(char::is_control),
            "Profile name must contain 1–48 printable characters"
        );
        let mut c = self.config.lock().unwrap();
        topology::validate(&c.layout)?;
        anyhow::ensure!(
            c.profiles.len() < 20 || c.profiles.contains_key(name),
            "Keep at most 20 layout profiles"
        );
        let mut next = c.clone();
        next.profiles
            .insert(name.trim().into(), next.layout.clone());
        config::save(&self.config_dir, &next)?;
        *c = next;
        self.update_routing(&c);
        Ok(())
    }
    pub fn load_profile(&self, name: &str) -> Result<()> {
        let mut layout = self
            .config
            .lock()
            .unwrap()
            .profiles
            .get(name)
            .cloned()
            .context("Unknown layout profile")?;
        let state = self.state.lock().unwrap();
        topology::reconcile_device(&mut layout, &state.local);
        for peer in state.peers.values().filter(|p| p.connected) {
            topology::reconcile_device(&mut layout, &peer.device);
        }
        drop(state);
        self.set_layout(layout)
    }
    pub fn delete_profile(&self, name: &str) -> Result<()> {
        let mut c = self.config.lock().unwrap();
        let mut next = c.clone();
        anyhow::ensure!(
            next.profiles.remove(name).is_some(),
            "Unknown layout profile"
        );
        config::save(&self.config_dir, &next)?;
        *c = next;
        self.update_routing(&c);
        Ok(())
    }
    pub fn peer_permissions(
        &self,
        id: &str,
        input: bool,
        clipboard: bool,
        files: bool,
    ) -> Result<()> {
        let mut c = self.config.lock().unwrap();
        let mut next = c.clone();
        let p = next.peers.get_mut(id).context("Computer is not paired")?;
        p.allow_input = input;
        p.allow_clipboard = clipboard;
        p.allow_files = files;
        config::save(&self.config_dir, &next)?;
        *c = next;
        self.update_routing(&c);
        drop(c);
        self.release();
        if !files {
            self.cancel_peer_files(Some(id));
        }
        Ok(())
    }
    pub fn forget_peer(&self, id: &str) -> Result<()> {
        self.release();
        self.cancel_peer_files(Some(id));
        let mut c = self.config.lock().unwrap();
        let mut next = c.clone();
        next.peers.remove(id);
        next.settings.mac_alt_tab_peers.remove(id);
        next.layout.retain(|d| d.device_id != id);
        next.hop_keys.retain(|d| d.device_id != id);
        config::save(&self.config_dir, &next)?;
        *c = next;
        self.update_routing(&c);
        drop(c);
        self.channels.lock().unwrap().remove(id);
        if let Some(p) = self.state.lock().unwrap().peers.get_mut(id) {
            p.trusted = false;
            p.connected = false;
        }
        Ok(())
    }
    pub fn release(&self) {
        let (active, inbound, away) = {
            let mut s = self.state.lock().unwrap();
            s.edge = None;
            let away = s.away.take().is_some();
            s.shared_keys.clear();
            s.shortcut.reset();
            (s.active.take(), s.inbound.take(), away)
        };
        let active_was_remote = active.is_some();
        let _ = self.sync_forwarding();
        if let Some(a) = active {
            self.send(&a.peer, Message::Leave);
            let _ = self.backend.inject(&InputEvent::Move {
                x: a.return_x,
                y: a.return_y,
            });
        }
        let had_inbound = inbound.is_some();
        if let Some(id) = inbound {
            self.send(&id, Message::Leave);
        }
        if active_was_remote || had_inbound || away {
            self.announce_focus(&self.identity.id);
        }
        let _ = self.backend.inject(&InputEvent::ReleaseAll);
        if active_was_remote {
            self.restore_physical();
        }
    }
    pub fn shutdown(&self) {
        self.stop.send_replace(true);
        self.disconnect_controls();
        self.backend.stop_capture();
        self.decisions.lock().unwrap().clear();
    }
    fn sync_forwarding(&self) -> Result<()> {
        // Serialize cursor visibility with ownership. A release racing a hop
        // must not leave the source hidden after its active lease was cleared.
        let mut state = self.state.lock().unwrap();
        if state.active.is_none() {
            state.shortcut.reset();
        }
        self.backend
            .set_forwarding(state.active.is_some() || state.away.is_some())
    }
    fn disconnect_controls(&self) {
        self.release();
        let mut state = self.state.lock().unwrap();
        let mut channels = self.channels.lock().unwrap();
        channels.clear();
        state.layout_offers.clear();
        for peer in state.peers.values_mut() {
            peer.connected = false;
            peer.latency_ms = None;
        }
    }
    pub fn refresh_displays(&self) -> Result<bool> {
        let displays = match self.backend.displays() {
            Ok(displays) if !displays.is_empty() && displays.iter().all(|d| d.bounds.valid()) => {
                displays
            }
            _ => {
                // Preserve the saved arrangement, but never route into stale displays.
                self.displays_available.store(false, Ordering::SeqCst);
                self.input_available.store(false, Ordering::SeqCst);
                self.clear_physical_state();
                self.disconnect_controls();
                anyhow::bail!("No usable displays available; input sharing suspended");
            }
        };
        if self.state.lock().unwrap().local.displays == displays {
            self.displays_available.store(true, Ordering::SeqCst);
            self.input_available.store(
                self.capture_started.load(Ordering::SeqCst) && self.backend.input_available(),
                Ordering::SeqCst,
            );
            return Ok(false);
        }
        self.displays_available.store(false, Ordering::SeqCst);
        self.input_available.store(false, Ordering::SeqCst);
        self.disconnect_controls();
        let aliases = self.backend.display_id_aliases()?;
        let mut config = self.config.lock().unwrap();
        let mut next = config.clone();
        let mut local = self.state.lock().unwrap().local.clone();
        local.displays = displays;
        migrate_display_ids(&mut next, &local.id, &aliases);
        topology::reconcile_device(&mut next.layout, &local);
        topology::validate(&next.layout)?;
        config::save(&self.config_dir, &next)?;
        *config = next;
        self.update_routing(&config);
        let mut state = self.state.lock().unwrap();
        state.local = local;
        let display = &state.local.displays[0];
        let center = (
            display.bounds.x + display.bounds.width / 2.,
            display.bounds.y + display.bounds.height / 2.,
        );
        if !state.local.displays.iter().any(|d| {
            d.bounds
                .contains(state.local_position.0, state.local_position.1)
        }) {
            state.local_position = center;
        }
        drop(state);
        drop(config);
        self.displays_available.store(true, Ordering::SeqCst);
        self.input_available.store(
            self.capture_started.load(Ordering::SeqCst) && self.backend.input_available(),
            Ordering::SeqCst,
        );
        self.metadata_changed.send_modify(|n| *n += 1);
        self.note("Display configuration refreshed; input returned locally".into());
        Ok(true)
    }
    pub fn refresh_session(&self) {
        let now = SystemTime::now();
        let gap = {
            let mut clock = self.session_clock.lock().unwrap();
            let gap = now
                .duration_since(*clock)
                .map_or(true, |gap| gap > Duration::from_secs(3));
            *clock = now;
            gap
        };
        if gap {
            self.recover_session_gap();
        }
        let available = self.backend.session_available();
        let input = self.capture_started.load(Ordering::SeqCst)
            && self.backend.input_available()
            && self.displays_available.load(Ordering::SeqCst);
        let was_available = self.session_available.swap(available, Ordering::SeqCst);
        let had_input = self.input_available.swap(input, Ordering::SeqCst);
        if (was_available && !available) || (had_input && !input) {
            self.clear_physical_state();
            self.disconnect_controls();
            if !available {
                self.cancel_peer_files(None);
            }
            self.note("OS input/session unavailable; released input and suspended sharing".into());
        } else if !was_available && available {
            self.note("OS session available; paired computers can reconnect".into());
        }
    }
    /// Conservative recovery after sleep, scheduler stalls, or a wall-clock jump.
    fn clear_physical_state(&self) {
        let mut state = self.state.lock().unwrap();
        state.pressed.clear();
        state.buttons.clear();
        state.consumed.clear();
        state.shortcut.reset();
    }
    pub fn recover_session_gap(&self) {
        // Clear stale physical state before release can restore held modifiers.
        self.clear_physical_state();
        self.disconnect_controls();
        self.cancel_peer_files(None);
        self.note(
            "Session timing gap; released input and cancelled transfers before reconnecting".into(),
        );
    }
    fn start_session_monitor(self: &Arc<Self>) {
        let service = self.clone();
        tokio::spawn(async move {
            let mut stop = service.stop.subscribe();
            if *stop.borrow() {
                return;
            }
            let mut tick = tokio::time::interval(Duration::from_millis(500));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! { biased; _=stop.changed()=>break, _=tick.tick()=>service.refresh_session() }
            }
        });
    }
    fn start_display_refresh(self: &Arc<Self>) {
        let service = self.clone();
        tokio::spawn(async move {
            let mut stop = service.stop.subscribe();
            if *stop.borrow() {
                return;
            }
            let mut tick = tokio::time::interval(Duration::from_secs(2));
            loop {
                tokio::select! {
                    biased;
                    _ = stop.changed() => break,
                    _ = tick.tick() => {
                        let service = service.clone();
                        let _ = tokio::task::spawn_blocking(move || { if let Err(error) = service.refresh_displays() { service.note(format!("Display refresh: {error}")); } }).await;
                    }
                }
            }
        });
    }
    pub fn export_diagnostics(&self) -> String {
        let snapshot = self.snapshot();
        serde_json::to_string_pretty(&serde_json::json!({
            "version":snapshot.version,
            "platform":snapshot.local.platform,
            "permissions":snapshot.permissions,
            "port":snapshot.listening_port,
            "peer_count":snapshot.peers.len(),
            "connected_count":snapshot.peers.iter().filter(|p|p.connected).count(),
            "rtt_ms":snapshot.peers.iter().filter_map(|p|p.latency_ms).collect::<Vec<_>>(),
            "messages":snapshot.diagnostics,
            "keyboard":snapshot.keyboard_diagnostics,
            "scroll":snapshot.scroll_diagnostics,
            "control_queues":snapshot.control_queues,
            "local_controls_pointer":snapshot.controller_device==snapshot.local.id,
            "cursor_is_local":snapshot.active_device==snapshot.local.id
        }))
        .unwrap()
    }
    pub(crate) fn approved(&self, id: &str, key: &[u8]) -> Result<bool> {
        let c = self.config.lock().unwrap();
        if let Some(p) = c.peers.get(id) {
            anyhow::ensure!(
                hex::decode(&p.public_key)? == key,
                "Pinned key mismatch: unpair and compare identities before reconnecting"
            );
            Ok(true)
        } else {
            Ok(false)
        }
    }
    fn update_routing(&self, config: &Config) {
        // Native hooks must never wait for configuration serialization/fsync.
        self.routing.store(Arc::new(RoutingConfig::from(config)));
    }
    pub(crate) async fn open(
        self: &Arc<Self>,
        address: &str,
        purpose: &str,
    ) -> Result<(String, SecureReader, SecureWriter)> {
        anyhow::ensure!(!*self.stop.borrow(), "Engine has stopped");
        let address: SocketAddr = address
            .parse()
            .context("Enter a local IP address and port, such as 192.168.1.20:24810")?;
        anyhow::ensure!(
            local_ip(address.ip()),
            "Only local/private network addresses are supported"
        );
        let stream = tokio::time::timeout(Duration::from_secs(3), TcpStream::connect(address))
            .await
            .context("Connection timed out")??;
        let secure = SecureConnection::handshake(stream, &self.identity, true).await?;
        self.exchange(secure, address, purpose, None).await
    }
    async fn exchange(
        self: &Arc<Self>,
        secure: SecureConnection,
        address: SocketAddr,
        purpose: &str,
        expected_id: Option<&str>,
    ) -> Result<(String, SecureReader, SecureWriter)> {
        let key = secure.remote_key().to_vec();
        let id = config::device_id(&key);
        anyhow::ensure!(
            id != self.identity.id,
            "Cannot connect to this computer itself"
        );
        anyhow::ensure!(
            expected_id.is_none_or(|expected| expected == id),
            "Address belongs to a different computer"
        );
        let approved = self.approved(&id, &key)?;
        let (mut reader, mut writer) = secure.split();
        let local = self.state.lock().unwrap().local.clone();
        let hello = Message::Hello {
            version: 2,
            device: local,
            purpose: purpose.into(),
            port: self.listening_port,
            approved,
        };
        let remote = tokio::time::timeout(Duration::from_secs(5), async {
            writer.send(&hello).await?;
            reader.recv().await
        })
        .await
        .context("Peer introduction timed out")??;
        let Message::Hello {
            version,
            device,
            purpose: remote_purpose,
            port,
            approved: remote_approved,
        } = remote
        else {
            anyhow::bail!("Expected encrypted peer introduction")
        };
        anyhow::ensure!(
            version == 2,
            "Update both computers to version 0.1.2 or newer; protocol versions differ"
        );
        anyhow::ensure!(
            port > 1024
                && device.id == id
                && device.name.len() <= 256
                && device.platform.len() <= 32
                && device.displays.len() <= 32
                && device.displays.iter().all(|d| d.bounds.valid()
                    && !d.id.is_empty()
                    && d.id.len() < 256
                    && d.name.len() <= 256
                    && d.scale.is_finite()
                    && d.scale > 0.
                    && d.scale <= 16.),
            "Invalid peer introduction"
        );
        anyhow::ensure!(remote_purpose == purpose, "Connection purpose mismatch");
        {
            let mut s = self.state.lock().unwrap();
            anyhow::ensure!(
                s.peers.len() < 64 || s.peers.contains_key(&id),
                "Too many nearby computers; restart to clear stale candidates"
            );
            let previous = s.peers.get(&id);
            let connected = previous.is_some_and(|p| p.connected);
            let latency_ms = previous.and_then(|p| p.latency_ms);
            s.peers.insert(
                id.clone(),
                PeerStatus {
                    device: device.clone(),
                    address: address.to_string(),
                    public_key: Some(hex::encode(&key)),
                    safety_code: Some(config::safety_code(&self.identity.public, &key)),
                    trusted: approved,
                    connected,
                    latency_ms,
                    last_error: if !approved || !remote_approved {
                        Some("Compare the safety code and approve on both computers".into())
                    } else {
                        None
                    },
                },
            );
        }
        // An unapproved encrypted handshake may show its identity, but never receives data.
        anyhow::ensure!(
            approved && remote_approved,
            "Pairing pending: approve the matching safety code on both computers"
        );
        if purpose == "control" {
            let mut c = self.config.lock().unwrap();
            let mut next = c.clone();
            if let Some(p) = next.peers.get_mut(&id) {
                p.address = Some(address.to_string());
            }
            topology::reconcile_device(&mut next.layout, &device);
            config::save(&self.config_dir, &next)?;
            *c = next;
            self.update_routing(&c);
        }
        Ok((id, reader, writer))
    }
    pub async fn connect(self: &Arc<Self>, address: &str) -> Result<()> {
        let (id, reader, writer) = self.open(address, "control").await?;
        anyhow::ensure!(!*self.stop.borrow(), "Engine has stopped");
        self.start_control(id, reader, writer, true);
        Ok(())
    }
    async fn reconnect(self: &Arc<Self>, id: &str, address: &str) -> Result<()> {
        anyhow::ensure!(
            self.session_available.load(Ordering::SeqCst),
            "OS session unavailable"
        );
        let address: SocketAddr = address.parse()?;
        anyhow::ensure!(local_ip(address.ip()), "Only local addresses are supported");
        let stream =
            tokio::time::timeout(Duration::from_secs(3), TcpStream::connect(address)).await??;
        let secure = SecureConnection::handshake(stream, &self.identity, true).await?;
        let (id, reader, writer) = self.exchange(secure, address, "control", Some(id)).await?;
        self.start_control(id, reader, writer, true);
        Ok(())
    }
    async fn accept(
        self: &Arc<Self>,
        stream: TcpStream,
        address: SocketAddr,
        handshake_permit: tokio::sync::OwnedSemaphorePermit,
    ) -> Result<()> {
        let secure = SecureConnection::handshake(stream, &self.identity, false).await?;
        // Read purpose before replying; incoming source ports are ephemeral, so use
        // discovered/remembered listening address rather than source port for reconnect.
        let key = secure.remote_key().to_vec();
        let id = config::device_id(&key);
        let approved = self.approved(&id, &key)?;
        let (mut reader, mut writer) = secure.split();
        let hello = tokio::time::timeout(Duration::from_secs(5), reader.recv())
            .await
            .context("Introduction timed out")??;
        let Message::Hello {
            version,
            device,
            purpose,
            port,
            approved: remote_approved,
        } = hello
        else {
            anyhow::bail!("Expected peer introduction")
        };
        anyhow::ensure!(
            version == 2,
            "Update both computers to version 0.1.2 or newer; protocol versions differ"
        );
        anyhow::ensure!(
            port > 1024
                && device.id == id
                && id != self.identity.id
                && device.name.len() <= 256
                && device.platform.len() <= 32
                && device.displays.len() <= 32
                && device.displays.iter().all(|d| d.bounds.valid()
                    && !d.id.is_empty()
                    && d.id.len() < 256
                    && d.name.len() <= 256
                    && d.scale.is_finite()
                    && d.scale > 0.
                    && d.scale <= 16.),
            "Invalid peer identity/display list"
        );
        anyhow::ensure!(
            purpose == "control" || purpose == "data",
            "Unsupported connection purpose"
        );
        let local = self.state.lock().unwrap().local.clone();
        writer
            .send(&Message::Hello {
                version: 2,
                device: local,
                purpose: purpose.clone(),
                port: self.listening_port,
                approved,
            })
            .await?;
        let remembered = SocketAddr::new(address.ip(), port).to_string();
        {
            let mut s = self.state.lock().unwrap();
            anyhow::ensure!(
                s.peers.len() < 64 || s.peers.contains_key(&id),
                "Too many nearby computers; restart to clear stale candidates"
            );
            let previous = s.peers.get(&id);
            let addr = if purpose == "data" {
                previous.map(|p| p.address.clone()).unwrap_or(remembered)
            } else {
                remembered
            };
            let connected = previous.is_some_and(|p| p.connected);
            let latency_ms = previous.and_then(|p| p.latency_ms);
            s.peers.insert(
                id.clone(),
                PeerStatus {
                    device: device.clone(),
                    address: addr,
                    public_key: Some(hex::encode(&key)),
                    safety_code: Some(config::safety_code(&self.identity.public, &key)),
                    trusted: approved,
                    connected,
                    latency_ms,
                    last_error: if !approved || !remote_approved {
                        Some("Compare the safety code and approve on both computers".into())
                    } else {
                        None
                    },
                },
            );
        }
        if !(approved && remote_approved) {
            return Ok(());
        }
        // Pending bulk offers must not occupy handshake admission slots.
        drop(handshake_permit);
        if purpose == "control" {
            anyhow::ensure!(
                self.session_available.load(Ordering::SeqCst),
                "OS session unavailable"
            );
            {
                let mut c = self.config.lock().unwrap();
                let mut next = c.clone();
                if let Some(peer) = next.peers.get_mut(&id) {
                    peer.address = Some(SocketAddr::new(address.ip(), port).to_string());
                }
                topology::reconcile_device(&mut next.layout, &device);
                config::save(&self.config_dir, &next)?;
                *c = next;
                self.update_routing(&c);
            }
            self.start_control(id, reader, writer, false);
        } else {
            let _bulk = self
                .incoming_bulk
                .try_acquire()
                .context("Receiver is busy with four bulk transfers; retry shortly")?;
            self.receive_blob(id, reader, writer).await?;
        }
        Ok(())
    }
    fn start_control(
        self: &Arc<Self>,
        id: String,
        mut reader: SecureReader,
        mut writer: SecureWriter,
        outgoing: bool,
    ) {
        let tx = Arc::new(crate::outbound::Outbound::new(512));
        let rx = tx.clone();
        let (priority, mut urgent) = mpsc::unbounded_channel();
        let epoch = Arc::new(AtomicU64::new(0));
        let generation = uuid::Uuid::new_v4().to_string();
        // Both ends retain the connection initiated by the lower identity.
        let preferred = outgoing == (self.identity.id < id);
        {
            let mut state = self.state.lock().unwrap();
            let mut channels = self.channels.lock().unwrap();
            if *self.stop.borrow() {
                return;
            }
            if channels
                .get(&id)
                .is_some_and(|existing| existing.preferred || !preferred)
            {
                return;
            }
            channels.insert(
                id.clone(),
                Channel {
                    generation: generation.clone(),
                    preferred,
                    tx,
                    priority,
                    epoch: epoch.clone(),
                },
            );
            if let Some(p) = state.peers.get_mut(&id) {
                p.connected = true;
                p.last_error = None;
            }
        }
        let s = self.clone();
        tokio::spawn(async move {
            // Frame reads cannot be cancelled midway by a heartbeat or outbound
            // input. Keep the reader alive across control-loop select branches.
            let (incoming_tx, mut incoming) = mpsc::channel(64);
            let reader_task = tokio::spawn(async move {
                loop {
                    let message = reader.recv().await;
                    let failed = message.is_err();
                    if incoming_tx.send(message).await.is_err() || failed {
                        break;
                    }
                }
            });
            let mut stop = s.stop.subscribe();
            let start = Instant::now();
            let mut last_received = Instant::now();
            let mut heartbeat = tokio::time::interval(Duration::from_millis(300));
            let result:Result<()>=async{if *stop.borrow(){return Ok(());}loop{tokio::select!{
                biased;
                _=stop.changed()=>break,
                message=urgent.recv()=>match message{Some(m)=>{tokio::time::timeout(Duration::from_millis(750),writer.send(&m)).await.context("Emergency writer stalled")??;},None=>break},
                _=heartbeat.tick()=>{
                    anyhow::ensure!(s.routing.load().peers.contains_key(&id),"Computer was unpaired");
                    anyhow::ensure!(last_received.elapsed()<Duration::from_millis(1500),"Heartbeat expired; released input");
                    tokio::time::timeout(Duration::from_millis(750),writer.send(&Message::Ping(start.elapsed().as_micros() as u64))).await.context("Control writer stalled")??;
                },
                message=incoming.recv()=>{
                    if !s.channels.lock().unwrap().get(&id).is_some_and(|c|c.generation==generation){break;}
                    let message=message.context("Control reader closed")??;last_received=Instant::now();match message{
                        Message::Ping(n)=>tokio::time::timeout(Duration::from_millis(750),writer.send(&Message::Pong(n))).await.context("Heartbeat writer stalled")??,
                        Message::Pong(n)=>{let elapsed=start.elapsed().as_micros() as u64;anyhow::ensure!(n<=elapsed,"Invalid ping response");if let Some(p)=s.state.lock().unwrap().peers.get_mut(&id){p.latency_ms=Some((elapsed-n) as f64/1000.);}},
                        other=>s.receive_control(&id,other)?,
                    }
                },
                (sent_epoch,m)=rx.recv()=>{if sent_epoch!=epoch.load(Ordering::SeqCst){continue;}tokio::time::timeout(Duration::from_millis(750),writer.send(&m)).await.context("Input writer stalled")??;},
            }}Ok(())}.await;
            reader_task.abort();
            let needs_release = {
                let mut st = s.state.lock().unwrap();
                let mut channels = s.channels.lock().unwrap();
                if !channels
                    .get(&id)
                    .is_some_and(|c| c.generation == generation)
                {
                    return; // Superseded tasks must not disconnect a replacement.
                }
                channels.remove(&id);
                st.layout_offers.remove(&id);
                if let Some(p) = st.peers.get_mut(&id) {
                    p.connected = false;
                    p.last_error = result.as_ref().err().map(|e| e.to_string());
                }
                let shared = st
                    .shared_keys
                    .remove(&id)
                    .is_some_and(|keys| !keys.is_empty());
                shared
                    || st.active.as_ref().is_some_and(|a| a.peer == id)
                    || st.inbound.as_deref() == Some(&id)
                    || st.away.as_ref().is_some_and(|(owner, _)| owner == &id)
            };
            if needs_release {
                s.release();
            }
            if let Err(e) = result {
                s.note(format!("Connection closed: {e}"));
            }
        });
    }
    fn send(&self, id: &str, message: Message) -> bool {
        let scroll = matches!(message, Message::Input(InputEvent::Scroll { .. }));
        let keyboard = matches!(
            message,
            Message::Input(InputEvent::Key { .. }) | Message::SharedKey { .. }
        );
        let sent = self.channels.lock().unwrap().get(id).is_some_and(|c| {
            if matches!(
                message,
                Message::Leave | Message::Input(InputEvent::ReleaseAll)
            ) {
                c.epoch.fetch_add(1, Ordering::SeqCst);
                c.priority.send(message).is_ok()
            } else {
                c.tx.try_send((c.epoch.load(Ordering::SeqCst), message))
            }
        });
        if sent && keyboard {
            self.keyboard_forwarded.fetch_add(1, Ordering::Relaxed);
        }
        if sent && scroll {
            self.scroll_forwarded.fetch_add(1, Ordering::Relaxed);
        }
        sent
    }
    fn announce_focus(&self, target: &str) {
        let c = self.routing.load();
        let peers: Vec<_> = self
            .state
            .lock()
            .unwrap()
            .peers
            .iter()
            .filter(|(id, p)| p.connected && c.peers.get(*id).copied().unwrap_or(false))
            .map(|(id, _)| id.clone())
            .collect();
        for id in peers {
            self.send(
                &id,
                Message::CursorFocus {
                    device_id: target.into(),
                },
            );
        }
    }
    pub fn share_layout(&self) -> Result<usize> {
        let c = self.routing.load();
        anyhow::ensure!(
            !c.settings.paused,
            "Resume sharing before sending an arrangement"
        );
        let peers: Vec<_> = self
            .state
            .lock()
            .unwrap()
            .peers
            .iter()
            .filter(|(id, p)| p.connected && c.peers.get(*id).copied().unwrap_or(false))
            .map(|(id, _)| id.clone())
            .collect();
        let mut sent = 0;
        for id in peers {
            let layout = c
                .layout
                .iter()
                .filter(|l| l.device_id == self.identity.id || l.device_id == id)
                .cloned()
                .collect();
            if self.send(&id, Message::LayoutOffer { layout }) {
                sent += 1;
            }
        }
        anyhow::ensure!(sent > 0, "No connected computer allows input sharing");
        Ok(sent)
    }
    pub fn accept_layout(&self, peer: &str) -> Result<()> {
        let offer = self
            .state
            .lock()
            .unwrap()
            .layout_offers
            .get(peer)
            .cloned()
            .context("Arrangement offer expired")?;
        let mut layout = self.config.lock().unwrap().layout.clone();
        layout.retain(|l| l.device_id != self.identity.id && l.device_id != peer);
        layout.extend(offer);
        self.set_layout(layout)?;
        self.state.lock().unwrap().layout_offers.remove(peer);
        Ok(())
    }
    pub fn dismiss_layout(&self, peer: &str) {
        self.state.lock().unwrap().layout_offers.remove(peer);
    }
    fn receive_control(&self, id: &str, message: Message) -> Result<()> {
        let scroll = matches!(message, Message::Input(InputEvent::Scroll { .. }));
        if scroll {
            self.scroll_received.fetch_add(1, Ordering::Relaxed);
        }
        let keyboard = matches!(
            message,
            Message::Input(InputEvent::Key { .. }) | Message::SharedKey { .. }
        );
        if keyboard {
            self.keyboard_received.fetch_add(1, Ordering::Relaxed);
        }
        let allowed = {
            let c = self.routing.load();
            c.peers.get(id).is_some_and(|p| *p)
                && !c.settings.paused
                && self.session_available.load(Ordering::SeqCst)
                && self.input_available.load(Ordering::SeqCst)
        };
        match message {
            Message::SharedKey { code, down } => {
                anyhow::ensure!(
                    InputEvent::Key { code, down }.validate(),
                    "Invalid shared key"
                );
                if !allowed {
                    self.keyboard_rejected.fetch_add(1, Ordering::Relaxed);
                    return Ok(());
                }
                let c = self.routing.load();
                let mut st = self.state.lock().unwrap();
                let keys = st.shared_keys.entry(id.into()).or_default();
                if down {
                    keys.insert(code);
                } else {
                    keys.remove(&code);
                }
                let source = st
                    .peers
                    .get(id)
                    .map(|p| p.device.platform.as_str())
                    .unwrap_or("");
                let active = st.active.clone();
                let target = active
                    .as_ref()
                    .and_then(|a| st.peers.get(&a.peer))
                    .map(|p| p.device.platform.as_str())
                    .unwrap_or(st.local.platform.as_str());
                let event = InputEvent::Key {
                    code: topology::map_key(code, source, target, c.settings.map_modifiers),
                    down,
                };
                drop(st);
                if let Some(active) = active {
                    if c.peers.get(&active.peer).copied().unwrap_or(false)
                        && !self.send(&active.peer, Message::Input(event))
                    {
                        self.release();
                    }
                } else {
                    self.backend.inject(&event)?;
                    self.keyboard_applied.fetch_add(1, Ordering::Relaxed);
                }
            }
            Message::LayoutOffer { layout } => {
                anyhow::ensure!(allowed, "Input sharing disabled");
                topology::validate(&layout)?;
                let st = self.state.lock().unwrap();
                anyhow::ensure!(
                    layout.iter().all(|l| {
                        let device = if l.device_id == st.local.id {
                            Some(&st.local)
                        } else if l.device_id == id {
                            st.peers.get(id).map(|p| &p.device)
                        } else {
                            None
                        };
                        device.is_some_and(|d| d.displays.iter().any(|d| d.id == l.display_id))
                    }),
                    "Arrangement contains unknown displays"
                );
                anyhow::ensure!(
                    [st.local.id.as_str(), id]
                        .iter()
                        .all(|id| layout.iter().any(|l| l.device_id == *id && !l.hidden)),
                    "Arrangement must show both computers"
                );
                drop(st);
                self.state
                    .lock()
                    .unwrap()
                    .layout_offers
                    .insert(id.into(), layout);
            }
            Message::CursorFocus { device_id } => {
                if !allowed {
                    return Ok(());
                }
                let mut st = self.state.lock().unwrap();
                anyhow::ensure!(
                    device_id == st.local.id
                        || device_id == id
                        || st
                            .peers
                            .get(&device_id)
                            .is_some_and(|p| p.connected && p.trusted),
                    "Unknown cursor destination"
                );
                let old_active = st.active.take();
                let keep_inbound = device_id == st.local.id && st.inbound.as_deref() == Some(id);
                let old_inbound = if keep_inbound {
                    None
                } else {
                    st.inbound.take()
                };
                if device_id == st.local.id {
                    st.away = None;
                } else {
                    st.away = Some((id.into(), device_id));
                }
                st.edge = None;
                let departing = st.away.is_some();
                drop(st);
                if departing || old_active.is_some() || old_inbound.is_some() {
                    self.backend.inject(&InputEvent::ReleaseAll)?;
                }
                for previous in old_active.map(|a| a.peer).into_iter().chain(old_inbound) {
                    if previous != id {
                        self.send(&previous, Message::Leave);
                    }
                }
                self.sync_forwarding()?;
            }
            Message::Enter { display_id, x, y } => {
                anyhow::ensure!(allowed, "Input sharing disabled for this computer");
                let valid = {
                    let s = self.state.lock().unwrap();
                    s.local
                        .displays
                        .iter()
                        .any(|d| d.id == display_id && d.bounds.contains(x, y))
                };
                anyhow::ensure!(valid, "Invalid cursor entry");
                let (same_owner, old_active, old_inbound) = {
                    let mut st = self.state.lock().unwrap();
                    let same = st.inbound.as_deref() == Some(id);
                    let old = (st.active.take(), st.inbound.take());
                    st.inbound = Some(id.into());
                    st.away = None;
                    st.edge = None;
                    (same, old.0, old.1)
                };
                if !same_owner {
                    self.backend.inject(&InputEvent::ReleaseAll)?;
                }
                // Never echo Leave to the new controller: it would cancel its new lease.
                for previous in old_active.map(|a| a.peer).into_iter().chain(old_inbound) {
                    if previous != id {
                        self.send(&previous, Message::Leave);
                    }
                }
                self.sync_forwarding()?;
                self.backend.inject(&InputEvent::Move { x, y })?;
            }
            Message::Input(event) => {
                if scroll && !event.validate() {
                    self.scroll_rejected.fetch_add(1, Ordering::Relaxed);
                }
                anyhow::ensure!(event.validate(), "Invalid input event");
                let owner = self.state.lock().unwrap().inbound.as_deref() == Some(id);
                if allowed && owner {
                    if let InputEvent::Move { x, y } = &event {
                        anyhow::ensure!(
                            self.state
                                .lock()
                                .unwrap()
                                .local
                                .displays
                                .iter()
                                .any(|d| d.bounds.contains(*x, *y)),
                            "Cursor outside native display"
                        );
                    }
                    if let Err(error) = self.backend.inject(&event) {
                        if scroll {
                            self.scroll_rejected.fetch_add(1, Ordering::Relaxed);
                        }
                        return Err(error);
                    }
                    if scroll {
                        self.scroll_applied.fetch_add(1, Ordering::Relaxed);
                    }
                    if keyboard {
                        self.keyboard_applied.fetch_add(1, Ordering::Relaxed);
                    }
                } else {
                    if keyboard {
                        self.keyboard_rejected.fetch_add(1, Ordering::Relaxed);
                    }
                    if scroll {
                        self.scroll_rejected.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
            Message::Leave => {
                let mut s = self.state.lock().unwrap();
                let inbound = s.inbound.as_deref() == Some(id);
                if inbound {
                    s.inbound = None;
                }
                let active = s.active.as_ref().filter(|a| a.peer == id).cloned();
                if active.is_some() {
                    s.active = None;
                }
                drop(s);
                if inbound || active.is_some() {
                    let _ = self.backend.inject(&InputEvent::ReleaseAll);
                }
                if let Some(a) = active {
                    let _ = self.sync_forwarding();
                    let _ = self.backend.inject(&InputEvent::Move {
                        x: a.return_x,
                        y: a.return_y,
                    });
                    self.restore_physical();
                }
            }
            _ => anyhow::bail!("Message not allowed on the input channel"),
        }
        Ok(())
    }
    pub fn capture(&self, event: CapturedEvent) -> bool {
        if matches!(event, CapturedEvent::Input(InputEvent::Scroll { .. })) {
            self.scroll_captured.fetch_add(1, Ordering::Relaxed);
        }
        if matches!(event, CapturedEvent::Input(InputEvent::Key { .. })) {
            self.keyboard_captured.fetch_add(1, Ordering::Relaxed);
        }
        if matches!(event, CapturedEvent::Input(InputEvent::ReleaseAll)) {
            let mut state = self.state.lock().unwrap();
            state.pressed.clear();
            state.buttons.clear();
            state.consumed.clear();
            state.shortcut.reset();
            drop(state);
            self.release();
            return false;
        }
        if !self.session_available.load(Ordering::SeqCst)
            || !self.input_available.load(Ordering::SeqCst)
            || *self.stop.borrow()
        {
            return false;
        }
        let c = self.routing.load();
        let mut s = self.state.lock().unwrap();
        if let CapturedEvent::Input(InputEvent::Key { code, down }) = &event {
            if *down {
                s.pressed.insert(*code);
            } else {
                s.pressed.remove(code);
            }
            if *code == 41
                && *down
                && s.pressed.iter().any(|k| *k == 224 || *k == 228)
                && s.pressed.iter().any(|k| *k == 226 || *k == 230)
            {
                drop(s);
                self.release();
                return true;
            }
            if s.consumed.contains(code) {
                if !down {
                    s.consumed.remove(code);
                }
                return true;
            }
            if *down && !c.settings.paused {
                if let Some(binding) = c
                    .hop_keys
                    .iter()
                    .find(|b| b.keys.iter().all(|k| s.pressed.contains(k)))
                    .cloned()
                {
                    for k in &binding.keys {
                        s.consumed.insert(*k);
                    }
                    drop(s);
                    for code in &binding.keys {
                        let _ = self.backend.inject(&InputEvent::Key {
                            code: *code,
                            down: false,
                        });
                    }
                    if let Err(e) = self.jump_to_display(&binding.device_id, &binding.display_id) {
                        self.note(format!("Hop shortcut: {e}"));
                    }
                    return true;
                }
            }
        }
        if let CapturedEvent::Input(InputEvent::Button { button, down }) = &event {
            if *down {
                s.buttons.insert(*button);
            } else {
                s.buttons.remove(button);
            }
        }
        if c.settings.paused {
            return false;
        }
        // Only physical events reach capture; injected events are filtered by adapters.
        // A local controller can reclaim an idle computer without a stuck cursor.
        if let CapturedEvent::Input(InputEvent::Key { code, down }) = &event {
            if let Some((owner, _)) = &s.away {
                let owner = owner.clone();
                let message = Message::SharedKey {
                    code: *code,
                    down: *down,
                };
                drop(s);
                if self.send(&owner, message) {
                    return true;
                }
                self.release();
                return false;
            }
            // A receiving computer's own keyboard targets the same visible local cursor.
            if s.inbound.is_some() {
                return false;
            }
        }
        let reclaim = s.away.take().is_some() || s.inbound.is_some();
        if reclaim {
            drop(s);
            let _ = self.sync_forwarding();
            s = self.state.lock().unwrap();
        }
        if let Some(owner) = s.inbound.take() {
            drop(s);
            let _ = self.backend.inject(&InputEvent::ReleaseAll);
            self.send(&owner, Message::Leave);
            s = self.state.lock().unwrap();
        }
        if reclaim {
            drop(s);
            // Leave increments the queue epoch; announce after it so this
            // ownership update cannot be invalidated as stale input.
            self.announce_focus(&self.identity.id);
            s = self.state.lock().unwrap();
        }
        match event {
            CapturedEvent::Move { x, y, dx, dy } => {
                if dx != 0. || dy != 0. {
                    s.last_motion = (dx, dy);
                }
                let motion = if dx == 0. && dy == 0. {
                    s.last_motion
                } else {
                    (dx, dy)
                };
                let (from, wx, wy, return_x, return_y) = if let Some(a) = &s.active {
                    let Some(from) = c
                        .layout
                        .iter()
                        .find(|l| l.device_id == a.peer && l.display_id == a.display)
                    else {
                        drop(s);
                        self.release();
                        return false;
                    };
                    let (nx, ny) = from.rect.clamp(
                        a.x + dx * c.settings.sensitivity,
                        a.y + dy * c.settings.sensitivity,
                    );
                    (from.clone(), nx, ny, a.return_x, a.return_y)
                } else {
                    let Some(display) = s
                        .local
                        .displays
                        .iter()
                        .find(|d| d.bounds.contains(x, y))
                        .or_else(|| {
                            s.local.displays.iter().min_by(|a, b| {
                                let dist = |d: &Display| {
                                    let (cx, cy) = d.bounds.clamp(x, y);
                                    (cx - x).powi(2) + (cy - y).powi(2)
                                };
                                dist(a).total_cmp(&dist(b))
                            })
                        })
                    else {
                        return false;
                    };
                    let Some(from) = c.layout.iter().find(|l| {
                        l.device_id == s.local.id && l.display_id == display.id && !l.hidden
                    }) else {
                        return false;
                    };
                    let (cx, cy) = display.bounds.clamp(x, y);
                    let (wx, wy) = topology::to_workspace(&from.rect, &display.bounds, cx, cy);
                    s.local_position = (cx, cy);
                    (from.clone(), wx, wy, cx, cy)
                };
                let next = if c.settings.cursor_locked || c.settings.game_mode {
                    None
                } else {
                    topology::adjacent(&c.layout, &from, wx, wy, motion.0, motion.1)
                        .filter(|next| s.active.is_some() || next.device_id != s.local.id)
                        .cloned()
                };
                if let Some(next) = next {
                    let online = next.device_id == s.local.id
                        || s.peers.get(&next.device_id).is_some_and(|p| p.connected)
                            && c.peers.get(&next.device_id).is_some_and(|p| *p);
                    if online {
                        let edge_id = format!("{}:{}", next.device_id, next.display_id);
                        if s.edge.as_ref().is_none_or(|(id, _)| id != &edge_id) {
                            s.edge = Some((edge_id, Instant::now()));
                        }
                        if s.edge.as_ref().unwrap().1.elapsed()
                            >= Duration::from_millis(c.settings.edge_delay_ms)
                        {
                            let (nx, ny) = next
                                .rect
                                .clamp(wx + motion.0.signum() * 3., wy + motion.1.signum() * 3.);
                            let native = if next.device_id == s.local.id {
                                s.local
                                    .displays
                                    .iter()
                                    .find(|d| d.id == next.display_id)
                                    .cloned()
                            } else {
                                s.peers
                                    .get(&next.device_id)
                                    .and_then(|p| {
                                        p.device.displays.iter().find(|d| d.id == next.display_id)
                                    })
                                    .cloned()
                            };
                            if let Some(native) = native {
                                let (px, py) =
                                    topology::to_native(&next.rect, &native.bounds, nx, ny);
                                let old = s.active.take();
                                s.edge = None;
                                if next.device_id == s.local.id {
                                    drop(s);
                                    let _ = self.sync_forwarding();
                                    if let Some(a) = old {
                                        self.send(&a.peer, Message::Leave);
                                    }
                                    let _ = self.backend.inject(&InputEvent::Move { x: px, y: py });
                                    self.announce_focus(&self.identity.id);
                                    self.restore_physical();
                                    return true;
                                }
                                let peer = next.device_id.clone();
                                let same_peer = old.as_ref().is_some_and(|a| a.peer == peer);
                                if !self.queue_enter_locked(
                                    &s,
                                    &peer,
                                    &next.display_id,
                                    px,
                                    py,
                                    !same_peer,
                                ) {
                                    // Retain the old active lease for release,
                                    // so a failed handoff cleans up both sides.
                                    s.active = old;
                                    drop(s);
                                    self.send(&peer, Message::Leave);
                                    self.release();
                                    return false;
                                }
                                if !same_peer {
                                    s.shortcut.reset();
                                }
                                s.active = Some(Active {
                                    peer: peer.clone(),
                                    display: next.display_id.clone(),
                                    x: nx,
                                    y: ny,
                                    return_x,
                                    return_y,
                                });
                                let local_display = s
                                    .local
                                    .displays
                                    .iter()
                                    .find(|d| d.bounds.contains(return_x, return_y))
                                    .unwrap_or(&s.local.displays[0]);
                                let park_x =
                                    local_display.bounds.x + local_display.bounds.width / 2.;
                                let park_y =
                                    local_display.bounds.y + local_display.bounds.height / 2.;
                                drop(s);
                                let old_was_local = old.is_none();
                                if let Some(a) = old {
                                    if !same_peer {
                                        self.send(&a.peer, Message::Leave);
                                    }
                                }
                                if old_was_local {
                                    self.release_physical_on_source();
                                }
                                if !self.backend.manages_cursor_parking() {
                                    let _ = self.backend.inject(&InputEvent::Move {
                                        x: park_x,
                                        y: park_y,
                                    });
                                }
                                if let Err(e) = self.sync_forwarding() {
                                    self.note(format!("Cannot suppress local input: {e}"));
                                    self.release();
                                    return false;
                                }
                                self.announce_focus(&peer);
                                return true;
                            }
                        }
                    }
                } else {
                    s.edge = None;
                }
                if let Some(a) = s.active.as_mut() {
                    a.x = wx;
                    a.y = wy;
                    let peer = a.peer.clone();
                    let display = a.display.clone();
                    let native = s
                        .peers
                        .get(&peer)
                        .and_then(|p| p.device.displays.iter().find(|d| d.id == display))
                        .cloned();
                    drop(s);
                    if let Some(native) = native {
                        let (px, py) = topology::to_native(&from.rect, &native.bounds, wx, wy);
                        if self.send(&peer, Message::Input(InputEvent::Move { x: px, y: py })) {
                            return true;
                        }
                    }
                    self.release();
                    return false;
                }
                false
            }
            CapturedEvent::Input(input) => {
                if let Some(a) = &s.active {
                    let id = a.peer.clone();
                    let target = s
                        .peers
                        .get(&id)
                        .map(|p| p.device.platform.clone())
                        .unwrap_or_default();
                    let source = s.local.platform.clone();
                    let enabled = c.settings.mac_alt_tab
                        && c.settings.mac_alt_tab_peers.get(&id).copied().unwrap_or(true);
                    let left_alt = s.pressed.contains(&226);
                    let control = s.pressed.contains(&224) || s.pressed.contains(&228);
                    let right_alt = s.pressed.contains(&230);
                    let events = s.shortcut.route(
                        input,
                        topology::ShortcutContext {
                            source: &source,
                            target: &target,
                            enabled,
                            map_modifiers: c.settings.map_modifiers,
                            left_alt_down: left_alt,
                            control_down: control,
                            right_alt_down: right_alt,
                        },
                    );
                    // Publish the whole modifier transition under the same
                    // ownership lock used by the Windows keyboard/mouse hooks.
                    // Otherwise a concurrent event could slip between the
                    // synthetic Option-up, Command-down and Tab-down messages.
                    // Sending only enqueues to the bounded local control queue;
                    // it never waits for the peer or performs network I/O.
                    for event in events {
                        if !self.send(&id, Message::Input(event)) {
                            drop(s);
                            self.release();
                            return false;
                        }
                    }
                    true
                } else {
                    false
                }
            }
            CapturedEvent::Unsupported => s.active.is_some(),
        }
    }
    fn physical(&self) -> (Vec<u16>, Vec<u8>) {
        let s = self.state.lock().unwrap();
        (
            s.pressed
                .iter()
                .copied()
                .filter(|k| *k != 57 && *k != 41 && !s.consumed.contains(k))
                .collect(),
            s.buttons.iter().copied().collect(),
        )
    }
    fn release_physical_on_source(&self) {
        let (keys, buttons) = self.physical();
        for code in keys {
            let _ = self.backend.inject(&InputEvent::Key { code, down: false });
        }
        for button in buttons {
            let _ = self.backend.inject(&InputEvent::Button {
                button,
                down: false,
            });
        }
    }
    fn restore_physical(&self) {
        let (keys, buttons) = self.physical();
        for code in keys {
            let _ = self.backend.inject(&InputEvent::Key { code, down: true });
        }
        for button in buttons {
            let _ = self
                .backend
                .inject(&InputEvent::Button { button, down: true });
        }
    }
    // Enter and the pre-handoff held state are one publication barrier. Taking
    // the held snapshot later could resend a newly captured first key-down.
    fn queue_enter_locked(
        &self,
        state: &State,
        id: &str,
        display: &str,
        x: f64,
        y: f64,
        held: bool,
    ) -> bool {
        if !self.send(
            id,
            Message::Enter {
                display_id: display.into(),
                x,
                y,
            },
        ) {
            return false;
        }
        if !held {
            return true;
        }
        let enabled = self.routing.load().settings.map_modifiers;
        let source = &state.local.platform;
        let target = state
            .peers
            .get(id)
            .map(|p| p.device.platform.as_str())
            .unwrap_or("");
        for code in state
            .pressed
            .iter()
            .copied()
            .filter(|k| *k != 57 && *k != 41 && !state.consumed.contains(k))
        {
            if !self.send(
                id,
                Message::Input(InputEvent::Key {
                    code: topology::map_key(code, source, target, enabled),
                    down: true,
                }),
            ) {
                return false;
            }
        }
        for button in state.buttons.iter().copied() {
            if !self.send(
                id,
                Message::Input(InputEvent::Button { button, down: true }),
            ) {
                return false;
            }
        }
        true
    }
    fn reconnect_candidates(&self) -> Vec<(String, Vec<String>)> {
        let config = self.config.lock().unwrap();
        let state = self.state.lock().unwrap();
        config
            .peers
            .iter()
            .filter(|(id, _)| !state.peers.get(*id).is_some_and(|p| p.connected))
            .map(|(id, peer)| {
                let mut addresses = Vec::new();
                for address in state
                    .peers
                    .get(id)
                    .map(|p| &p.address)
                    .into_iter()
                    .chain(peer.address.iter())
                    .chain(state.discovered.get(id).into_iter().flatten())
                {
                    if !address.is_empty() && !addresses.contains(address) {
                        addresses.push(address.clone());
                    }
                }
                (id.clone(), addresses)
            })
            .collect()
    }
    fn discovered_peer(&self, id: &str, name: &str, platform: &str, addresses: Vec<String>) {
        if id == self.identity.id
            || id.len() != 32
            || !id.bytes().all(|b| b.is_ascii_hexdigit())
            || addresses.is_empty()
        {
            return;
        }
        let trusted = self.config.lock().unwrap().peers.contains_key(id);
        let mut state = self.state.lock().unwrap();
        if state.peers.len() >= 64 && !state.peers.contains_key(id) {
            return;
        }
        state.discovered.insert(id.into(), addresses.clone());
        // mDNS is only a hint. Never overwrite authenticated endpoint/metadata.
        if let Some(peer) = state.peers.get_mut(id) {
            if peer.public_key.is_none() {
                peer.address = addresses[0].clone();
                peer.device.name = name.chars().take(64).collect();
                peer.device.platform = platform.chars().take(32).collect();
            }
        } else {
            state.peers.insert(
                id.into(),
                PeerStatus {
                    device: Device {
                        id: id.into(),
                        name: name.chars().take(64).collect(),
                        platform: platform.chars().take(32).collect(),
                        displays: vec![],
                    },
                    address: addresses[0].clone(),
                    public_key: None,
                    safety_code: None,
                    trusted,
                    connected: false,
                    latency_ms: None,
                    last_error: None,
                },
            );
        }
    }
    fn start_reconnect(self: &Arc<Self>) {
        let s = self.clone();
        tokio::spawn(async move {
            let mut stop = s.stop.subscribe();
            if *stop.borrow() {
                return;
            }
            let mut tick = tokio::time::interval(Duration::from_secs(3));
            loop {
                tokio::select! {
                    biased;
                    _ = stop.changed() => break,
                    _ = tick.tick() => {
                        for (id, addresses) in s.reconnect_candidates() {
                            if id < s.identity.id { continue; }
                            for address in addresses {
                                let result = tokio::select! {
                                    biased;
                                    _ = stop.changed() => return,
                                    result = s.reconnect(&id, &address) => result,
                                };
                                match result {
                                    Ok(()) => break,
                                    Err(e) => if let Some(peer) = s.state.lock().unwrap().peers.get_mut(&id) { peer.last_error = Some(e.to_string()); },
                                }
                            }
                        }
                    }
                }
            }
        });
    }
    fn advertise(&self, daemon: &mdns_sd::ServiceDaemon) -> Result<()> {
        let local = self.state.lock().unwrap().local.clone();
        let props = [
            ("id", local.id.as_str()),
            ("name", local.name.as_str()),
            ("platform", local.platform.as_str()),
        ];
        let info = mdns_sd::ServiceInfo::new(
            "_hoppingcursor._tcp.local.",
            &local.id,
            &format!("hop-{}.local.", local.id),
            "",
            self.listening_port,
            &props[..],
        )?
        .enable_addr_auto();
        daemon.register(info)?;
        Ok(())
    }
    fn start_discovery(self: &Arc<Self>) {
        let daemon = match mdns_sd::ServiceDaemon::new() {
            Ok(d) => d,
            Err(e) => {
                self.note(format!(
                    "Discovery unavailable: {e}. Connect by local address."
                ));
                return;
            }
        };
        let kind = "_hoppingcursor._tcp.local.";
        if let Err(error) = self.advertise(&daemon) {
            self.note(format!("Discovery advertisement: {error}"));
        }
        let receiver = match daemon.browse(kind) {
            Ok(r) => r,
            Err(e) => {
                self.note(format!("Discovery browse: {e}"));
                return;
            }
        };
        let s = self.clone();
        tokio::spawn(async move {
            let mut stop = s.stop.subscribe();
            if *stop.borrow() {
                return;
            }
            let mut changed = s.metadata_changed.subscribe();
            loop {
                tokio::select! {_=stop.changed()=>break,_=changed.changed()=>{
                    if let Err(error) = s.advertise(&daemon) { s.note(format!("Discovery advertisement: {error}")); }
                },event=receiver.recv_async()=>match event{
                    Ok(mdns_sd::ServiceEvent::ServiceResolved(info))=>{
                        let Some(id)=info.get_property_val_str("id")else{continue;};if id==s.identity.id||id.len()!=32{continue;}
                        if info.get_port() <= 1024 { continue; }
                        let mut addresses = info.get_addresses().iter().filter(|ip| ip.is_ipv4() && local_ip(**ip) && !ip.is_loopback())
                            .map(|ip| SocketAddr::new(*ip, info.get_port()).to_string()).collect::<Vec<_>>();
                        addresses.sort(); addresses.truncate(8);
                        s.discovered_peer(id, info.get_property_val_str("name").unwrap_or("Nearby computer"), info.get_property_val_str("platform").unwrap_or("unknown"), addresses);
                    },Ok(_)=>{},Err(_)=>break,
                }}
            }
            let _ = daemon.shutdown();
        });
    }
    fn start_clipboard(self: &Arc<Self>) {
        let s = self.clone();
        tokio::spawn(async move {
            let mut stop = s.stop.subscribe();
            if *stop.borrow() {
                return;
            }
            let mut last_revision = None;
            let mut tick = tokio::time::interval(Duration::from_millis(700));
            loop {
                tokio::select! {_=stop.changed()=>break,_=tick.tick()=>{
                    let settings=s.config.lock().unwrap().settings.clone();if settings.paused||!s.session_available.load(Ordering::SeqCst)||!(settings.clipboard_text||settings.clipboard_html||settings.clipboard_images||settings.clipboard_files){continue;}
                    if s.clipboard_retry.swap(false, Ordering::SeqCst) { last_revision = None; }
                    let revision=s.backend.clipboard_revision().map(|n|(n,settings.clipboard_text,settings.clipboard_html,settings.clipboard_images,settings.clipboard_files));if revision.is_some()&&revision==last_revision{continue;}
                    if settings.clipboard_files {
                        let backend=s.backend.clone();
                        if let Ok(Ok(mut paths))=tokio::task::spawn_blocking(move||backend.read_file_clipboard()).await {
                            paths.sort();paths.dedup();
                            if !paths.is_empty() {
                                let hash=crate::transfer::file_list_hash(&paths);
                                let changed={let mut previous=s.file_clipboard_hash.lock().unwrap();if *previous==hash {false} else {*previous=hash;true}};
                                if changed && paths.len()<=100 {
                                    let peers={let c=s.config.lock().unwrap();let st=s.state.lock().unwrap();st.peers.iter().filter(|(id,p)|p.connected&&c.peers.get(*id).is_some_and(|p|p.allow_clipboard&&p.allow_files)).map(|(id,_)|id.clone()).collect::<Vec<_>>()};
                                    for peer in peers {let service=s.clone();let paths=paths.clone();tokio::spawn(async move {if service.send_file_clipboard(&peer,paths).await.is_err(){service.note("Copied files could not be shared. Check transfer consent, file limits and use regular files.".into());}});}
                                }
                                last_revision=revision;
                                continue;
                            }
                        }
                    }
                    let backend=s.backend.clone();let parts=match tokio::task::spawn_blocking(move||backend.read_clipboard()).await{Ok(Ok(p))=>filter_clipboard(p,&settings),_=>continue};last_revision=revision;if parts.is_empty(){continue;}
                    let Ok(data)=serde_json::to_vec(&parts)else{continue;};if data.len()>8*1024*1024{continue;}let hash=Sha256::digest(&data).to_vec();
                    let changed={let mut st=s.state.lock().unwrap();if st.clipboard_hash==hash{false}else{st.clipboard_hash=hash.clone();true}};if !changed{continue;}
                    let peers={let c=s.config.lock().unwrap();let st=s.state.lock().unwrap();st.peers.iter().filter(|(id,p)|p.connected&&c.peers.get(*id).is_some_and(|p|p.allow_clipboard)).map(|(id,_)|id.clone()).collect::<Vec<_>>()};
                    for id in peers{let s=s.clone();let data=data.clone();let hash=hash.clone();tokio::spawn(async move{if let Err(e)=s.send_clipboard(&id,data).await{
                        let mut state=s.state.lock().unwrap();if state.clipboard_hash==hash{state.clipboard_hash.clear();}drop(state);
                        s.clipboard_retry.store(true,Ordering::SeqCst);s.note(format!("Clipboard transfer: {e}"));
                    }});}
                }}
            }
        });
    }
}
fn migrate_display_ids(config: &mut Config, device_id: &str, aliases: &BTreeMap<String, String>) {
    for layout in std::iter::once(&mut config.layout).chain(config.profiles.values_mut()) {
        for entry in layout
            .iter_mut()
            .filter(|entry| entry.device_id == device_id)
        {
            if let Some(id) = aliases.get(&entry.display_id) {
                entry.display_id = id.clone();
            }
        }
        let mut seen = BTreeSet::new();
        layout.retain(|entry| seen.insert((entry.device_id.clone(), entry.display_id.clone())));
    }
    for key in config
        .hop_keys
        .iter_mut()
        .filter(|key| key.device_id == device_id)
    {
        if let Some(id) = aliases.get(&key.display_id) {
            key.display_id = id.clone();
        }
    }
}
pub(crate) fn filter_clipboard(
    parts: Vec<ClipboardPart>,
    settings: &Settings,
) -> Vec<ClipboardPart> {
    parts
        .into_iter()
        .filter(|p| match p.mime.as_str() {
            "text/plain" => settings.clipboard_text,
            "text/html" => settings.clipboard_html,
            "image/png" => settings.clipboard_images,
            _ => false,
        })
        .collect()
}
pub(crate) fn local_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || (ip.segments()[0] & 0xfe00) == 0xfc00
                || (ip.segments()[0] & 0xffc0) == 0xfe80
                || ip.to_ipv4_mapped().is_some_and(|a| local_ip(IpAddr::V4(a)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::Headless;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn start_test(
        dir: PathBuf,
        port: Option<u16>,
        backend: Arc<dyn Platform>,
    ) -> Result<Arc<Service>> {
        Service::start_with_options(
            dir,
            port,
            backend,
            StartOptions {
                discovery: false,
                protect_identity: false,
                loopback_only: true,
            },
        )
        .await
    }
    #[tokio::test]
    async fn discovery_cannot_replace_verified_address_or_connect_unpaired_devices() {
        let dir = tempfile::tempdir().unwrap();
        let service = start_test(dir.path().into(), Some(0), Arc::new(Headless::default()))
            .await
            .unwrap();
        let peer_dir = tempfile::tempdir().unwrap();
        let peer = Identity::from_config(&config::load(peer_dir.path()).unwrap()).unwrap();
        let key = hex::encode(&peer.public);
        service.discovered_peer(
            &peer.id,
            "Nearby",
            "headless",
            vec!["127.0.0.1:29999".into()],
        );
        assert!(
            service.reconnect_candidates().is_empty(),
            "Discovery must not start unrequested pairing handshakes"
        );
        service
            .state
            .lock()
            .unwrap()
            .peers
            .get_mut(&peer.id)
            .unwrap()
            .public_key = Some(key.clone());
        service.approve_peer(&peer.id, &key).unwrap();
        {
            let mut state = service.state.lock().unwrap();
            let status = state.peers.get_mut(&peer.id).unwrap();
            status.address = "127.0.0.1:28888".into();
            status.connected = true;
        }
        service.discovered_peer(
            &peer.id,
            "Unverified rename",
            "unknown",
            vec!["127.0.0.1:27777".into()],
        );
        let snapshot = service.snapshot();
        let status = snapshot
            .peers
            .iter()
            .find(|p| p.device.id == peer.id)
            .unwrap();
        assert_eq!(status.address, "127.0.0.1:28888");
        assert_eq!(status.device.name, "Nearby");
        assert!(service.reconnect_candidates().is_empty());
        service.shutdown();
    }

    #[tokio::test]
    async fn native_capture_does_not_wait_for_configuration_writes() {
        let dir = tempfile::tempdir().unwrap();
        let service = start_test(dir.path().into(), Some(0), Arc::new(Headless::default()))
            .await
            .unwrap();
        let config_write = service.config.lock().unwrap();
        let (done, result) = std::sync::mpsc::channel();
        let source = service.clone();
        let hook = std::thread::spawn(move || {
            done.send(source.capture(CapturedEvent::Move {
                x: 100.,
                y: 100.,
                dx: 1.,
                dy: 0.,
            }))
            .unwrap();
        });
        let completed = result.recv_timeout(Duration::from_millis(500));
        drop(config_write);
        hook.join().unwrap();
        service.shutdown();
        assert!(
            matches!(completed, Ok(false)),
            "Native hooks must remain responsive while configuration is locked"
        );
    }

    #[tokio::test]
    async fn remote_lease_is_queued_before_native_handoff_for_jump_and_edge() {
        // Fake only the OS boundary. The service, queues and ownership are real.
        struct ParkingBackend {
            inner: Arc<Headless>,
            native_parking: bool,
        }
        impl Platform for ParkingBackend {
            fn name(&self) -> &str {
                self.inner.name()
            }
            fn displays(&self) -> Result<Vec<Display>> {
                self.inner.displays()
            }
            fn permissions(&self) -> Permissions {
                self.inner.permissions()
            }
            fn start_capture(&self, cb: crate::platform::CaptureCallback) -> Result<()> {
                self.inner.start_capture(cb)
            }
            fn stop_capture(&self) {
                self.inner.stop_capture()
            }
            fn set_forwarding(&self, forwarding: bool) -> Result<()> {
                self.inner.set_forwarding(forwarding)
            }
            fn manages_cursor_parking(&self) -> bool {
                self.native_parking
            }
            fn inject(&self, event: &InputEvent) -> Result<()> {
                self.inner.inject(event)
            }
            fn read_clipboard(&self) -> Result<Vec<ClipboardPart>> {
                self.inner.read_clipboard()
            }
            fn write_clipboard(&self, parts: &[ClipboardPart]) -> Result<()> {
                self.inner.write_clipboard(parts)
            }
        }
        for (edge, native_parking) in [(false, false), (true, false), (false, true), (true, true)] {
            let dir = tempfile::tempdir().unwrap();
            let backend = Arc::new(Headless::default());
            let source = start_test(
                dir.path().into(),
                Some(0),
                Arc::new(ParkingBackend {
                    inner: backend.clone(),
                    native_parking,
                }),
            )
            .await
            .unwrap();
            let mut remote = source.snapshot().local;
            remote.id = "test-peer".into();
            {
                let mut c = source.config.lock().unwrap();
                c.settings.edge_delay_ms = 0;
                c.peers.insert(
                    remote.id.clone(),
                    TrustedPeer {
                        public_key: "synthetic".into(),
                        name: "test".into(),
                        address: None,
                        allow_input: true,
                        allow_clipboard: false,
                        allow_files: false,
                    },
                );
                source.update_routing(&c);
            }
            source.state.lock().unwrap().peers.insert(
                remote.id.clone(),
                PeerStatus {
                    device: remote.clone(),
                    address: "127.0.0.1:1".into(),
                    public_key: None,
                    safety_code: None,
                    trusted: true,
                    connected: true,
                    latency_ms: None,
                    last_error: None,
                },
            );
            let local = source.snapshot().local;
            source
                .set_layout(vec![
                    LayoutDisplay {
                        device_id: local.id,
                        display_id: local.displays[0].id.clone(),
                        rect: Rect {
                            x: 0.,
                            y: 0.,
                            width: 1920.,
                            height: 1080.,
                        },
                        hidden: false,
                    },
                    LayoutDisplay {
                        device_id: remote.id.clone(),
                        display_id: remote.displays[0].id.clone(),
                        rect: Rect {
                            x: 1920.,
                            y: 0.,
                            width: 1920.,
                            height: 1080.,
                        },
                        hidden: false,
                    },
                ])
                .unwrap();
            let normal = Arc::new(crate::outbound::Outbound::new(32));
            let (priority, _urgent) = mpsc::unbounded_channel();
            source.channels.lock().unwrap().insert(
                remote.id.clone(),
                Channel {
                    generation: "test".into(),
                    preferred: true,
                    tx: normal.clone(),
                    priority,
                    epoch: Arc::new(AtomicU64::new(0)),
                },
            );
            let ready = Arc::new(AtomicBool::new(false));
            let observed = ready.clone();
            *backend.forwarding_probe.lock().unwrap() = Some(Arc::new(move |forwarding| {
                if forwarding {
                    while let Some((_, message)) = normal.try_recv() {
                        if matches!(message, Message::Enter { .. }) {
                            observed.store(true, Ordering::SeqCst);
                        }
                    }
                }
            }));
            if edge {
                assert!(source.capture(CapturedEvent::Move {
                    x: 1919.,
                    y: 500.,
                    dx: 5.,
                    dy: 0.
                }));
            } else {
                source
                    .jump_to_display(&remote.id, &remote.displays[0].id)
                    .unwrap();
            }
            let queued = ready.load(Ordering::SeqCst);
            let parking_moves = backend
                .events
                .lock()
                .unwrap()
                .iter()
                .filter(|e| matches!(e, InputEvent::Move { .. }))
                .count();
            source.shutdown();
            assert!(queued, "Enter must be queued before native forwarding exposes the new active owner (edge={edge})");
            assert_eq!(
                parking_moves,
                usize::from(!native_parking),
                "Native parking must not leave a queued mouse move after return (edge={edge}, native={native_parking})"
            );
        }
    }

    #[tokio::test]
    async fn scroll_diagnostics_count_denied_and_invalid_input_without_exporting_deltas() {
        let dir = tempfile::tempdir().unwrap();
        let backend = Arc::new(Headless::default());
        let service = start_test(dir.path().into(), Some(0), backend.clone())
            .await
            .unwrap();
        assert!(!service.capture(CapturedEvent::Input(InputEvent::Scroll { x: 0., y: 0.125 })));
        service
            .receive_control(
                "unpaired",
                Message::Input(InputEvent::Scroll { x: 0., y: 0.125 }),
            )
            .unwrap();
        service.state.lock().unwrap().inbound = Some("unpaired".into());
        service
            .receive_control(
                "unpaired",
                Message::Input(InputEvent::Scroll { x: -0.25, y: 0. }),
            )
            .unwrap();
        assert!(service
            .receive_control(
                "unpaired",
                Message::Input(InputEvent::Scroll { x: f64::NAN, y: 0. })
            )
            .is_err());
        let stats = service.snapshot().scroll_diagnostics;
        assert_eq!(
            (
                stats.captured,
                stats.forwarded,
                stats.received,
                stats.applied,
                stats.rejected
            ),
            (1, 0, 3, 0, 3)
        );
        assert!(!backend
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|event| matches!(event, InputEvent::Scroll { .. })));
        let exported = service.export_diagnostics();
        let data: serde_json::Value = serde_json::from_str(&exported).unwrap();
        assert!(data.get("public_key").is_none());
        assert!(data["scroll"].get("x").is_none());
        assert!(data["scroll"].get("y").is_none());
        assert!(
            !exported.contains("0.125")
                && !exported.contains("unpaired")
                && !exported.contains("NaN")
        );
        service.shutdown();
    }

    #[tokio::test]
    async fn emergency_release_bypasses_saturated_input_and_invalidates_stale_keys() {
        let dir = tempfile::tempdir().unwrap();
        let service = start_test(dir.path().into(), Some(0), Arc::new(Headless::default()))
            .await
            .unwrap();
        let tx = Arc::new(crate::outbound::Outbound::new(1));
        let normal = tx.clone();
        let (priority, mut urgent) = mpsc::unbounded_channel();
        let epoch = Arc::new(AtomicU64::new(0));
        service.channels.lock().unwrap().insert(
            "test-peer".into(),
            Channel {
                generation: "test".into(),
                preferred: true,
                tx,
                priority,
                epoch: epoch.clone(),
            },
        );
        assert!(service.send(
            "test-peer",
            Message::Input(InputEvent::Key {
                code: 4,
                down: true
            })
        ));
        assert!(!service.send(
            "test-peer",
            Message::Input(InputEvent::Key {
                code: 5,
                down: true
            })
        ));
        assert!(service.send("test-peer", Message::Leave));
        assert!(matches!(urgent.recv().await, Some(Message::Leave)));
        let (queued_epoch, _) = normal.recv().await;
        assert_ne!(
            queued_epoch,
            epoch.load(Ordering::SeqCst),
            "Queued key-down must be discarded after recovery"
        );
        service.shutdown();
    }

    async fn fragment(
        mut from: tokio::net::tcp::OwnedReadHalf,
        mut to: tokio::net::tcp::OwnedWriteHalf,
    ) {
        let mut buffer = [0u8; 1024];
        while let Ok(n) = from.read(&mut buffer).await {
            if n == 0 {
                break;
            }
            for (index, byte) in buffer[..n].iter().enumerate() {
                if to.write_all(&[*byte]).await.is_err() {
                    return;
                }
                if index == 0 {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                } else {
                    tokio::task::yield_now().await;
                }
            }
        }
    }

    #[tokio::test]
    async fn fragmented_reads_survive_outbound_input_and_heartbeats() {
        let da = tempfile::tempdir().unwrap();
        let db = tempfile::tempdir().unwrap();
        let backend_b = Arc::new(Headless::default());
        let a = start_test(da.path().into(), Some(0), Arc::new(Headless::default()))
            .await
            .unwrap();
        let b = start_test(db.path().into(), Some(0), backend_b.clone())
            .await
            .unwrap();
        for (source, target) in [(&a, &b), (&b, &a)] {
            let mut c = source.config.lock().unwrap();
            c.peers.insert(
                target.identity.id.clone(),
                TrustedPeer {
                    public_key: hex::encode(&target.identity.public),
                    name: "test".into(),
                    address: None,
                    allow_input: true,
                    allow_clipboard: false,
                    allow_files: false,
                },
            );
            source.update_routing(&c);
            source.state.lock().unwrap().peers.insert(
                target.identity.id.clone(),
                PeerStatus {
                    device: target.snapshot().local,
                    address: "127.0.0.1:1".into(),
                    public_key: Some(hex::encode(&target.identity.public)),
                    safety_code: None,
                    trusted: true,
                    connected: false,
                    latency_ms: None,
                    last_error: None,
                },
            );
        }
        let server = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = proxy.local_addr().unwrap();
        let upstream_address = server.local_addr().unwrap();
        let relay = tokio::spawn(async move {
            let (downstream, _) = proxy.accept().await.unwrap();
            let upstream = TcpStream::connect(upstream_address).await.unwrap();
            let (dr, dw) = downstream.into_split();
            let (ur, uw) = upstream.into_split();
            tokio::join!(fragment(dr, uw), fragment(ur, dw));
        });
        let a_stream = TcpStream::connect(address).await.unwrap();
        let (b_stream, _) = server.accept().await.unwrap();
        let (ca, cb) = tokio::join!(
            SecureConnection::handshake(a_stream, &a.identity, true),
            SecureConnection::handshake(b_stream, &b.identity, false)
        );
        let (ar, aw) = ca.unwrap().split();
        let (br, bw) = cb.unwrap().split();
        a.start_control(b.identity.id.clone(), ar, aw, true);
        b.start_control(a.identity.id.clone(), br, bw, false);
        assert!(a.send(
            &b.identity.id,
            Message::Enter {
                display_id: "headless-main".into(),
                x: 500.,
                y: 500.
            }
        ));
        for n in 0..24 {
            assert!(a.send(
                &b.identity.id,
                Message::Input(InputEvent::Key {
                    code: 4,
                    down: n % 2 == 0
                })
            ));
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(
            a.snapshot()
                .peers
                .iter()
                .any(|p| p.device.id == b.identity.id && p.connected),
            "{:?}",
            a.snapshot().diagnostics
        );
        assert!(
            b.snapshot()
                .peers
                .iter()
                .any(|p| p.device.id == a.identity.id && p.connected),
            "{:?}",
            b.snapshot().diagnostics
        );
        assert!(backend_b.events.lock().unwrap().iter().any(|e| matches!(
            e,
            InputEvent::Key {
                code: 4,
                down: true
            }
        )));
        let start = Instant::now();
        let before_release = backend_b.events.lock().unwrap().len();
        assert!(a.send(&b.identity.id, Message::Leave));
        loop {
            if backend_b.events.lock().unwrap()[before_release..].contains(&InputEvent::ReleaseAll)
            {
                break;
            }
            assert!(
                start.elapsed() < Duration::from_secs(2),
                "Remote held keys were not released"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        a.shutdown();
        b.shutdown();
        relay.abort();
    }
}
