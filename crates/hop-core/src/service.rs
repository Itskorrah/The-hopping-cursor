use crate::{
    config::{self, Config, Identity},
    crypto::{SecureConnection, SecureReader, SecureWriter},
    model::*,
    platform::Platform,
    topology,
};
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot, watch},
};

pub(crate) struct State {
    pub local: Device,
    pub peers: PeerMap,
    pub transfers: BTreeMap<String, TransferStatus>,
    pub diagnostics: Vec<String>,
    active: Option<Active>,
    inbound: Option<String>,
    pressed: BTreeSet<u16>,
    edge: Option<(String, Instant)>,
    buttons: BTreeSet<u8>,
    last_motion: (f64, f64),
    pub clipboard_hash: Vec<u8>,
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
    tx: mpsc::Sender<Message>,
}
pub struct Service {
    pub(crate) identity: Identity,
    pub(crate) config: Mutex<Config>,
    pub(crate) config_dir: PathBuf,
    pub(crate) state: Mutex<State>,
    pub(crate) backend: Arc<dyn Platform>,
    channels: Mutex<BTreeMap<String, Channel>>,
    pub(crate) decisions: Mutex<BTreeMap<String, oneshot::Sender<bool>>>,
    pub(crate) cancelled: Mutex<BTreeSet<String>>,
    pub(crate) stop: watch::Sender<bool>,
    pub listening_port: u16,
}
impl Service {
    pub async fn start(
        dir: PathBuf,
        port_override: Option<u16>,
        backend: Arc<dyn Platform>,
    ) -> Result<Arc<Self>> {
        let mut config = config::load(&dir)?;
        let identity = Identity::from_config(&config)?;
        let listener =
            TcpListener::bind(("0.0.0.0", port_override.unwrap_or(config.settings.port)))
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
        // Preserve existing arrangement; newly attached displays are placed to the right.
        for d in &local.displays {
            if !config
                .layout
                .iter()
                .any(|l| l.device_id == local.id && l.display_id == d.id)
            {
                let x = config
                    .layout
                    .iter()
                    .filter(|l| !l.hidden)
                    .map(|l| l.rect.x + l.rect.width)
                    .fold(0., f64::max);
                config.layout.push(LayoutDisplay {
                    device_id: local.id.clone(),
                    display_id: d.id.clone(),
                    rect: Rect {
                        x,
                        y: 0.,
                        width: d.bounds.width,
                        height: d.bounds.height,
                    },
                    hidden: false,
                });
            }
        }
        config::save(&dir, &config)?;
        let (stop, _) = watch::channel(false);
        let this = Arc::new(Self {
            identity,
            config: Mutex::new(config),
            config_dir: dir,
            state: Mutex::new(State {
                local,
                peers: BTreeMap::new(),
                transfers: BTreeMap::new(),
                diagnostics: vec![],
                active: None,
                inbound: None,
                pressed: BTreeSet::new(),
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
            listening_port,
        });
        let weak = Arc::downgrade(&this);
        if let Err(e) = this.backend.start_capture(Arc::new(move |event| {
            weak.upgrade().is_some_and(|s| s.capture(event))
        })) {
            this.note(format!("Input capture: {e}"));
        }
        this.note(format!(
            "Listening on TCP {listening_port}; encrypted connections only"
        ));
        let s = this.clone();
        tokio::spawn(async move {
            let mut stop = s.stop.subscribe();
            let permits = Arc::new(tokio::sync::Semaphore::new(32));
            loop {
                tokio::select! {_ = stop.changed()=>break,result=listener.accept()=>match result{Ok((stream,address))=>{
                    if !local_ip(address.ip()){continue;}let Ok(permit)=permits.clone().try_acquire_owned() else {continue;};
                    let s=s.clone();tokio::spawn(async move{let _permit=permit;if let Err(e)=s.accept(stream,address).await{s.note(format!("Incoming connection: {e}"));}});
                },Err(e)=>s.note(format!("Listener: {e}"))}}
            }
        });
        this.start_discovery();
        this.start_reconnect();
        this.start_clipboard();
        Ok(this)
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
            profiles: c.profiles.keys().cloned().collect(),
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
            active_device: s
                .active
                .as_ref()
                .map(|a| a.peer.clone())
                .or_else(|| s.inbound.clone())
                .unwrap_or_else(|| s.local.id.clone()),
            listening_port: self.listening_port,
            diagnostics: s.diagnostics.clone(),
            version: env!("CARGO_PKG_VERSION").into(),
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
        let should_release = settings.paused;
        let mut c = self.config.lock().unwrap();
        let mut next = c.clone();
        next.settings = settings;
        config::save(&self.config_dir, &next)?;
        *c = next;
        self.state.lock().unwrap().local.name = c.settings.device_name.clone();
        drop(c);
        if should_release {
            self.release();
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
        Ok(())
    }
    pub fn load_profile(&self, name: &str) -> Result<()> {
        let layout = self
            .config
            .lock()
            .unwrap()
            .profiles
            .get(name)
            .cloned()
            .context("Unknown layout profile")?;
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
        drop(c);
        self.release();
        Ok(())
    }
    pub fn forget_peer(&self, id: &str) -> Result<()> {
        self.release();
        let mut c = self.config.lock().unwrap();
        let mut next = c.clone();
        next.peers.remove(id);
        next.layout.retain(|d| d.device_id != id);
        config::save(&self.config_dir, &next)?;
        *c = next;
        drop(c);
        self.channels.lock().unwrap().remove(id);
        if let Some(p) = self.state.lock().unwrap().peers.get_mut(id) {
            p.trusted = false;
            p.connected = false;
        }
        Ok(())
    }
    pub fn release(&self) {
        let (active, inbound) = {
            let mut s = self.state.lock().unwrap();
            s.edge = None;
            (s.active.take(), s.inbound.take())
        };
        let active_was_remote = active.is_some();
        if let Some(a) = active {
            self.send(&a.peer, Message::Input(InputEvent::ReleaseAll));
            self.send(&a.peer, Message::Leave);
            let _ = self.backend.inject(&InputEvent::Move {
                x: a.return_x,
                y: a.return_y,
            });
        }
        if let Some(id) = inbound {
            self.send(&id, Message::Leave);
        }
        let _ = self.backend.inject(&InputEvent::ReleaseAll);
        if active_was_remote {
            self.restore_physical();
        }
    }
    pub fn shutdown(&self) {
        self.release();
        self.backend.stop_capture();
        let _ = self.stop.send(true);
        self.channels.lock().unwrap().clear();
        self.decisions.lock().unwrap().clear();
    }
    pub fn export_diagnostics(&self) -> String {
        let snapshot = self.snapshot();
        serde_json::to_string_pretty(&serde_json::json!({"version":snapshot.version,"platform":snapshot.local.platform,"permissions":snapshot.permissions,"port":snapshot.listening_port,"peer_count":snapshot.peers.len(),"connected_count":snapshot.peers.iter().filter(|p|p.connected).count(),"rtt_ms":snapshot.peers.iter().filter_map(|p|p.latency_ms).collect::<Vec<_>>(),"messages":snapshot.diagnostics})).unwrap()
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
    pub(crate) async fn open(
        self: &Arc<Self>,
        address: &str,
        purpose: &str,
    ) -> Result<(String, SecureReader, SecureWriter)> {
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
        self.exchange(secure, address, purpose, true).await
    }
    async fn exchange(
        self: &Arc<Self>,
        secure: SecureConnection,
        address: SocketAddr,
        purpose: &str,
        outgoing: bool,
    ) -> Result<(String, SecureReader, SecureWriter)> {
        let key = secure.remote_key().to_vec();
        let id = config::device_id(&key);
        anyhow::ensure!(
            id != self.identity.id,
            "Cannot connect to this computer itself"
        );
        let approved = self.approved(&id, &key)?;
        let (mut reader, mut writer) = secure.split();
        let local = self.state.lock().unwrap().local.clone();
        let hello = Message::Hello {
            version: 1,
            device: local,
            purpose: purpose.into(),
            port: self.listening_port,
            approved,
        };
        let remote = tokio::time::timeout(Duration::from_secs(5), async {
            if outgoing {
                writer.send(&hello).await?;
                reader.recv().await
            } else {
                let remote = reader.recv().await?;
                writer.send(&hello).await?;
                Ok(remote)
            }
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
            version == 1
                && port > 1024
                && device.id == id
                && device.name.len() <= 256
                && device.displays.len() <= 32
                && device.displays.iter().all(|d| d.bounds.valid()
                    && d.id.len() < 256
                    && d.scale.is_finite()
                    && d.scale > 0.),
            "Invalid peer introduction"
        );
        anyhow::ensure!(remote_purpose == purpose, "Connection purpose mismatch");
        {
            let mut s = self.state.lock().unwrap();
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
            for d in &device.displays {
                if !next
                    .layout
                    .iter()
                    .any(|l| l.device_id == id && l.display_id == d.id)
                {
                    let x = next
                        .layout
                        .iter()
                        .filter(|d| !d.hidden)
                        .map(|d| d.rect.x + d.rect.width)
                        .fold(0., f64::max);
                    next.layout.push(LayoutDisplay {
                        device_id: id.clone(),
                        display_id: d.id.clone(),
                        rect: Rect {
                            x,
                            y: 0.,
                            width: d.bounds.width,
                            height: d.bounds.height,
                        },
                        hidden: false,
                    });
                }
            }
            config::save(&self.config_dir, &next)?;
            *c = next;
        }
        Ok((id, reader, writer))
    }
    pub async fn connect(self: &Arc<Self>, address: &str) -> Result<()> {
        let (id, reader, writer) = self.open(address, "control").await?;
        self.start_control(id, reader, writer);
        Ok(())
    }
    async fn accept(self: &Arc<Self>, stream: TcpStream, address: SocketAddr) -> Result<()> {
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
            version == 1
                && device.id == id
                && id != self.identity.id
                && device.name.len() <= 256
                && device.displays.len() <= 32
                && device.displays.iter().all(|d| d.bounds.valid()
                    && d.id.len() < 256
                    && d.scale.is_finite()
                    && d.scale > 0.),
            "Invalid peer identity/display list"
        );
        anyhow::ensure!(
            purpose == "control" || purpose == "data",
            "Unsupported connection purpose"
        );
        let local = self.state.lock().unwrap().local.clone();
        writer
            .send(&Message::Hello {
                version: 1,
                device: local,
                purpose: purpose.clone(),
                port: self.listening_port,
                approved,
            })
            .await?;
        let remembered = SocketAddr::new(address.ip(), port).to_string();
        {
            let mut s = self.state.lock().unwrap();
            let previous = s.peers.get(&id);
            let addr = if purpose=="data" {previous.map(|p|p.address.clone()).unwrap_or(remembered)} else {remembered};
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
        if purpose == "control" {
            {
                let mut c = self.config.lock().unwrap();
                let mut next = c.clone();
                for d in &device.displays {
                    if !next
                        .layout
                        .iter()
                        .any(|l| l.device_id == id && l.display_id == d.id)
                    {
                        let x = next
                            .layout
                            .iter()
                            .filter(|d| !d.hidden)
                            .map(|d| d.rect.x + d.rect.width)
                            .fold(0., f64::max);
                        next.layout.push(LayoutDisplay {
                            device_id: id.clone(),
                            display_id: d.id.clone(),
                            rect: Rect {
                                x,
                                y: 0.,
                                width: d.bounds.width,
                                height: d.bounds.height,
                            },
                            hidden: false,
                        });
                    }
                }
                config::save(&self.config_dir, &next)?;
                *c = next;
            }
            self.start_control(id, reader, writer);
        } else {
            self.receive_blob(id, reader, writer).await?;
        }
        Ok(())
    }
    fn start_control(
        self: &Arc<Self>,
        id: String,
        mut reader: SecureReader,
        mut writer: SecureWriter,
    ) {
        let (tx, mut rx) = mpsc::channel(512);
        let generation = uuid::Uuid::new_v4().to_string();
        {
            let mut channels = self.channels.lock().unwrap();
            if channels.contains_key(&id) {
                return;
            }
            channels.insert(
                id.clone(),
                Channel {
                    generation: generation.clone(),
                    tx,
                },
            );
        }
        if let Some(p) = self.state.lock().unwrap().peers.get_mut(&id) {
            p.connected = true;
            p.last_error = None;
        }
        let s = self.clone();
        tokio::spawn(async move {
            let mut stop = s.stop.subscribe();
            let start = Instant::now();
            let mut last_received = Instant::now();
            let mut heartbeat = tokio::time::interval(Duration::from_millis(300));
            let result:Result<()>=async{loop{tokio::select!{
                _=stop.changed()=>break,
                _=heartbeat.tick()=>{
                    anyhow::ensure!(s.config.lock().unwrap().peers.contains_key(&id),"Computer was unpaired");
                    anyhow::ensure!(last_received.elapsed()<Duration::from_millis(1500),"Heartbeat expired; released input");
                    tokio::time::timeout(Duration::from_millis(750),writer.send(&Message::Ping(start.elapsed().as_micros() as u64))).await.context("Control writer stalled")??;
                },
                message=rx.recv()=>match message{Some(m)=>{tokio::time::timeout(Duration::from_millis(750),writer.send(&m)).await.context("Input writer stalled")??;},None=>break},
                message=reader.recv()=>{
                    let message=message?;last_received=Instant::now();match message{
                        Message::Ping(n)=>writer.send(&Message::Pong(n)).await?,
                        Message::Pong(n)=>{let elapsed=start.elapsed().as_micros() as u64;anyhow::ensure!(n<=elapsed,"Invalid ping response");if let Some(p)=s.state.lock().unwrap().peers.get_mut(&id){p.latency_ms=Some((elapsed-n) as f64/1000.);}},
                        other=>s.receive_control(&id,other)?,
                    }
                }
            }}Ok(())}.await;
            {
                let mut channels = s.channels.lock().unwrap();
                if channels
                    .get(&id)
                    .is_some_and(|c| c.generation == generation)
                {
                    channels.remove(&id);
                }
            }
            let needs_release = {
                let mut st = s.state.lock().unwrap();
                if let Some(p) = st.peers.get_mut(&id) {
                    p.connected = false;
                    p.last_error = result.as_ref().err().map(|e| e.to_string());
                }
                st.active.as_ref().is_some_and(|a| a.peer == id)
                    || st.inbound.as_deref() == Some(&id)
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
        self.channels
            .lock()
            .unwrap()
            .get(id)
            .is_some_and(|c| c.tx.try_send(message).is_ok())
    }
    fn receive_control(&self, id: &str, message: Message) -> Result<()> {
        let allowed = {
            let c = self.config.lock().unwrap();
            c.peers.get(id).is_some_and(|p| p.allow_input) && !c.settings.paused
        };
        match message {
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
                let same_owner = self.state.lock().unwrap().inbound.as_deref() == Some(id);
                if !same_owner {
                    self.release();
                } else {
                    let _ = self.backend.inject(&InputEvent::ReleaseAll);
                }
                self.state.lock().unwrap().inbound = Some(id.into());
                self.backend.inject(&InputEvent::Move { x, y })?;
            }
            Message::Input(event) => {
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
                    self.backend.inject(&event)?;
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
        let c = self.config.lock().unwrap().clone();
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
        if let Some(owner) = s.inbound.take() {
            drop(s);
            let _ = self.backend.inject(&InputEvent::ReleaseAll);
            self.send(&owner, Message::Leave);
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
                    (from.clone(), wx, wy, cx, cy)
                };
                let next = if c.settings.cursor_locked || c.settings.game_mode {
                    None
                } else {
                    topology::adjacent(&c.layout, &from, wx, wy, motion.0, motion.1).cloned()
                };
                if let Some(next) = next {
                    let online = next.device_id == s.local.id
                        || s.peers.get(&next.device_id).is_some_and(|p| p.connected)
                            && c.peers.get(&next.device_id).is_some_and(|p| p.allow_input);
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
                                    if let Some(a) = old {
                                        self.send(&a.peer, Message::Input(InputEvent::ReleaseAll));
                                        self.send(&a.peer, Message::Leave);
                                    }
                                    let _ = self.backend.inject(&InputEvent::Move { x: px, y: py });
                                    self.restore_physical();
                                    return true;
                                }
                                let peer = next.device_id.clone();
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
                                    self.send(&a.peer, Message::Input(InputEvent::ReleaseAll));
                                    self.send(&a.peer, Message::Leave);
                                }
                                if old_was_local {
                                    self.release_physical_on_source();
                                }
                                let _ = self.backend.inject(&InputEvent::Move {
                                    x: park_x,
                                    y: park_y,
                                });
                                if !self.send(
                                    &peer,
                                    Message::Enter {
                                        display_id: next.display_id,
                                        x: px,
                                        y: py,
                                    },
                                ) {
                                    self.release();
                                    return false;
                                }
                                self.forward_physical_to(&peer);
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
            CapturedEvent::Input(mut input) => {
                if let Some(a) = &s.active {
                    let id = a.peer.clone();
                    let target = s
                        .peers
                        .get(&id)
                        .map(|p| p.device.platform.clone())
                        .unwrap_or_default();
                    let source = s.local.platform.clone();
                    drop(s);
                    if let InputEvent::Key { code, .. } = &mut input {
                        *code =
                            topology::map_key(*code, &source, &target, c.settings.map_modifiers);
                    }
                    if self.send(&id, Message::Input(input)) {
                        true
                    } else {
                        self.release();
                        false
                    }
                } else {
                    false
                }
            }
        }
    }
    fn physical(&self) -> (Vec<u16>, Vec<u8>) {
        let s = self.state.lock().unwrap();
        (
            s.pressed
                .iter()
                .copied()
                .filter(|k| *k != 57 && *k != 41)
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
    fn forward_physical_to(&self, id: &str) {
        let enabled = self.config.lock().unwrap().settings.map_modifiers;
        let (source, target) = {
            let s = self.state.lock().unwrap();
            (
                s.local.platform.clone(),
                s.peers
                    .get(id)
                    .map(|p| p.device.platform.clone())
                    .unwrap_or_default(),
            )
        };
        let (keys, buttons) = self.physical();
        for code in keys {
            self.send(
                id,
                Message::Input(InputEvent::Key {
                    code: topology::map_key(code, &source, &target, enabled),
                    down: true,
                }),
            );
        }
        for button in buttons {
            self.send(
                id,
                Message::Input(InputEvent::Button { button, down: true }),
            );
        }
    }
    fn start_reconnect(self: &Arc<Self>) {
        let s = self.clone();
        tokio::spawn(async move {
            let mut stop = s.stop.subscribe();
            let mut tick = tokio::time::interval(Duration::from_secs(3));
            loop {
                tokio::select! {_=stop.changed()=>break,_=tick.tick()=>{
                    let peers={let c=s.config.lock().unwrap();let st=s.state.lock().unwrap();let mut candidates:BTreeMap<String,String>=c.peers.iter().filter_map(|(id,p)|p.address.clone().map(|a|(id.clone(),a))).collect();for(id,p)in &st.peers{candidates.insert(id.clone(),p.address.clone());}candidates.into_iter().filter(|(id,_)|id>&s.identity.id&&!st.peers.get(id).is_some_and(|p|p.connected)).collect::<Vec<_>>()};
                    for(id,address)in peers{let result=s.connect(&address).await;if let Err(e)=result{let mut st=s.state.lock().unwrap();if let Some(p)=st.peers.get_mut(&id){p.last_error=Some(e.to_string());}}}
                }}
            }
        });
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
        let local = self.state.lock().unwrap().local.clone();
        let kind = "_hoppingcursor._tcp.local.";
        let props = [
            ("id", local.id.as_str()),
            ("name", local.name.as_str()),
            ("platform", local.platform.as_str()),
        ];
        match mdns_sd::ServiceInfo::new(
            kind,
            &local.id,
            &format!("hop-{}.local.", local.id),
            "",
            self.listening_port,
            &props[..],
        )
        .map(|i| i.enable_addr_auto())
        .and_then(|i| daemon.register(i))
        {
            Ok(()) => {}
            Err(e) => self.note(format!("Discovery advertisement: {e}")),
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
            loop {
                tokio::select! {_=stop.changed()=>break,event=receiver.recv_async()=>match event{
                    Ok(mdns_sd::ServiceEvent::ServiceResolved(info))=>{
                        let Some(id)=info.get_property_val_str("id")else{continue;};if id==s.identity.id||id.len()!=32{continue;}
                        let Some(ip)=info.get_addresses().iter().find(|a|a.is_ipv4()&&local_ip(**a))else{continue;};let address=SocketAddr::new(*ip,info.get_port()).to_string();
                        let trusted=s.config.lock().unwrap().peers.contains_key(id);let mut st=s.state.lock().unwrap();if st.peers.len()>=64&&!st.peers.contains_key(id){continue;}if let Some(p)=st.peers.get_mut(id){p.address=address;}else{
                            st.peers.insert(id.into(),PeerStatus{device:Device{id:id.into(),name:info.get_property_val_str("name").unwrap_or("Nearby computer").chars().take(64).collect(),platform:info.get_property_val_str("platform").unwrap_or("unknown").chars().take(32).collect(),displays:vec![]},address,public_key:None,safety_code:None,trusted,connected:false,latency_ms:None,last_error:None});
                        }
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
            let mut tick = tokio::time::interval(Duration::from_millis(700));
            loop {
                tokio::select! {_=stop.changed()=>break,_=tick.tick()=>{
                    let settings=s.config.lock().unwrap().settings.clone();if settings.paused||!(settings.clipboard_text||settings.clipboard_html||settings.clipboard_images){continue;}
                    let backend=s.backend.clone();let parts=match tokio::task::spawn_blocking(move||backend.read_clipboard()).await{Ok(Ok(p))=>filter_clipboard(p,&settings),_=>continue};if parts.is_empty(){continue;}
                    let Ok(data)=serde_json::to_vec(&parts)else{continue;};if data.len()>8*1024*1024{continue;}let hash=Sha256::digest(&data).to_vec();
                    let changed={let mut st=s.state.lock().unwrap();if st.clipboard_hash==hash{false}else{st.clipboard_hash=hash;true}};if !changed{continue;}
                    let peers={let c=s.config.lock().unwrap();let st=s.state.lock().unwrap();st.peers.iter().filter(|(id,p)|p.connected&&c.peers.get(*id).is_some_and(|p|p.allow_clipboard)).map(|(id,_)|id.clone()).collect::<Vec<_>>()};
                    for id in peers{let s=s.clone();let data=data.clone();tokio::spawn(async move{if let Err(e)=s.send_clipboard(&id,data).await{s.note(format!("Clipboard transfer: {e}"));}});}
                }}
            }
        });
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
