use hop_core::{
    config, platform::Headless, CapturedEvent, ClipboardPart, InputEvent, LayoutDisplay, Rect,
    Service,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

struct Pair {
    a: Arc<Service>,
    b: Arc<Service>,
    pa: Arc<Headless>,
    pb: Arc<Headless>,
    _a: tempfile::TempDir,
    _b: tempfile::TempDir,
}
impl Drop for Pair {
    fn drop(&mut self) {
        self.a.shutdown();
        self.b.shutdown();
    }
}
async fn wait(mut check: impl FnMut() -> bool) {
    let start = Instant::now();
    while !check() {
        assert!(
            start.elapsed() < Duration::from_secs(8),
            "Condition timed out"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
async fn pair(auto_accept: bool) -> Pair {
    let da = tempfile::tempdir().unwrap();
    let db = tempfile::tempdir().unwrap();
    for d in [&da, &db] {
        let mut c = config::load(d.path()).unwrap();
        c.settings.auto_accept_files = auto_accept;
        c.settings.receive_directory = d.path().join("received").to_string_lossy().into();
        c.settings.edge_delay_ms = 0;
        config::save(d.path(), &c).unwrap();
    }
    let pa = Arc::new(Headless::default());
    let pb = Arc::new(Headless::default());
    let a = Service::start(da.path().to_owned(), Some(0), pa.clone())
        .await
        .unwrap();
    let b = Service::start(db.path().to_owned(), Some(0), pb.clone())
        .await
        .unwrap();
    let address = format!("127.0.0.1:{}", b.listening_port);
    assert!(a
        .connect(&address)
        .await
        .unwrap_err()
        .to_string()
        .contains("Pairing pending"));
    wait(|| {
        b.snapshot()
            .peers
            .iter()
            .any(|p| p.device.id == a.snapshot().local.id && p.public_key.is_some())
    })
    .await;
    let sa = a.snapshot();
    let sb = b.snapshot();
    let peer_a = sa
        .peers
        .iter()
        .find(|p| p.device.id == sb.local.id)
        .unwrap();
    let peer_b = sb
        .peers
        .iter()
        .find(|p| p.device.id == sa.local.id)
        .unwrap();
    assert!(!peer_a.connected);
    assert!(!peer_b.connected);
    assert_eq!(peer_a.safety_code, peer_b.safety_code);
    a.approve_peer(&sb.local.id, &sb.public_key).unwrap();
    b.approve_peer(&sa.local.id, &sa.public_key).unwrap();
    a.connect(&address).await.unwrap();
    wait(|| {
        a.snapshot().peers.iter().any(|p| p.connected)
            && b.snapshot().peers.iter().any(|p| p.connected)
    })
    .await;
    Pair {
        a,
        b,
        pa,
        pb,
        _a: da,
        _b: db,
    }
}

#[tokio::test]
async fn encrypted_stream_verifies_files_and_never_overwrites() {
    let p = pair(true).await;
    let content: Vec<u8> = (0..300_000).map(|n| (n % 251) as u8).collect();
    let path = p._a.path().join("notes.bin");
    tokio::fs::write(&path, &content).await.unwrap();
    let id = p.b.snapshot().local.id;
    p.a.send_file(&id, path.clone()).await.unwrap();
    p.a.send_file(&id, path).await.unwrap();
    let dest = p._b.path().join("received");
    assert_eq!(
        tokio::fs::read(dest.join("notes.bin")).await.unwrap(),
        content
    );
    assert_eq!(
        tokio::fs::read(dest.join("notes (1).bin")).await.unwrap(),
        content
    );
    assert!(p
        .b
        .snapshot()
        .transfers
        .iter()
        .all(|t| t.state == "complete" && t.bytes == t.size));
    assert!(!std::fs::read_dir(dest).unwrap().any(|p| p
        .unwrap()
        .path()
        .extension()
        .is_some_and(|e| e == "part")));
}
#[tokio::test]
async fn explicit_recipient_consent_is_required() {
    let p = pair(false).await;
    let path = p._a.path().join("consent.txt");
    tokio::fs::write(&path, b"private file").await.unwrap();
    let a = p.a.clone();
    let id = p.b.snapshot().local.id;
    let task = tokio::spawn(async move { a.send_file(&id, path).await });
    wait(|| {
        p.b.snapshot()
            .transfers
            .iter()
            .any(|t| t.state == "offered")
    })
    .await;
    assert!(!p._b.path().join("received/consent.txt").exists());
    let offer = p.b.snapshot().transfers[0].id.clone();
    p.b.respond_transfer(&offer, false).unwrap();
    assert!(task.await.unwrap().is_err());
    assert!(!p._b.path().join("received/consent.txt").exists());
    assert_eq!(p.b.snapshot().transfers[0].state, "declined");
}
#[tokio::test]
async fn held_keys_handoff_and_disconnect_return_input() {
    let p = pair(true).await;
    let a = p.a.snapshot();
    let b = p.b.snapshot();
    p.a.set_layout(vec![
        LayoutDisplay {
            device_id: a.local.id.clone(),
            display_id: a.local.displays[0].id.clone(),
            rect: Rect {
                x: 0.,
                y: 0.,
                width: 1920.,
                height: 1080.,
            },
            hidden: false,
        },
        LayoutDisplay {
            device_id: b.local.id.clone(),
            display_id: b.local.displays[0].id.clone(),
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
    assert!(!p.a.capture(CapturedEvent::Input(InputEvent::Key {
        code: 224,
        down: true
    })));
    assert!(p.a.capture(CapturedEvent::Move {
        x: 1919.,
        y: 500.,
        dx: 4.,
        dy: 0.
    }));
    wait(|| {
        p.pb.events.lock().unwrap().contains(&InputEvent::Key {
            code: 224,
            down: true,
        })
    })
    .await;
    assert!(p.pa.events.lock().unwrap().contains(&InputEvent::Key {
        code: 224,
        down: false
    }));
    assert_eq!(p.a.snapshot().active_device, b.local.id);
    p.b.shutdown();
    wait(|| p.a.snapshot().active_device == a.local.id).await;
    assert!(p.pa.events.lock().unwrap().contains(&InputEvent::Key {
        code: 224,
        down: true
    }));
}
#[tokio::test]
async fn emergency_chord_recovers_without_waiting_for_network() {
    let p = pair(true).await;
    let a = p.a.snapshot();
    let b = p.b.snapshot();
    let mut layout = a.layout.clone();
    for l in &mut layout {
        l.rect.x = if l.device_id == a.local.id { 0. } else { 1920. };
        l.rect.y = 0.;
    }
    p.a.set_layout(layout).unwrap();
    p.a.capture(CapturedEvent::Move {
        x: 1919.,
        y: 500.,
        dx: 4.,
        dy: 0.,
    });
    assert_eq!(p.a.snapshot().active_device, b.local.id);
    p.a.capture(CapturedEvent::Input(InputEvent::Key {
        code: 224,
        down: true,
    }));
    p.a.capture(CapturedEvent::Input(InputEvent::Key {
        code: 226,
        down: true,
    }));
    assert!(p.a.capture(CapturedEvent::Input(InputEvent::Key {
        code: 41,
        down: true
    })));
    assert_eq!(p.a.snapshot().active_device, a.local.id);
}
#[tokio::test]
async fn clipboard_is_transferred_without_bounce() {
    let p = pair(true).await;
    *p.pa.clipboard.lock().unwrap() = vec![ClipboardPart {
        mime: "text/plain".into(),
        data: "hello from the other computer 🖱️".as_bytes().to_vec(),
    }];
    wait(|| {
        p.pb.clipboard
            .lock()
            .unwrap()
            .first()
            .is_some_and(|p| p.data == "hello from the other computer 🖱️".as_bytes())
    })
    .await;
    tokio::time::sleep(Duration::from_millis(900)).await;
    assert_eq!(p.pa.clipboard.lock().unwrap().len(), 1);
    assert_eq!(p.pb.clipboard.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn revocation_blocks_transfers_and_clipboard() {
    let p = pair(true).await;
    let a = p.a.snapshot();
    let b = p.b.snapshot();
    p.b.forget_peer(&a.local.id).unwrap();
    let path = p._a.path().join("revoked.txt");
    tokio::fs::write(&path, b"must not arrive").await.unwrap();
    assert!(p.a.send_file(&b.local.id, path).await.is_err());
    assert!(!p._b.path().join("received/revoked.txt").exists());
}
#[tokio::test]
async fn rejects_identity_approval_race_and_nonlocal_addresses() {
    let p = pair(true).await;
    assert!(p
        .a
        .approve_peer(&p.b.snapshot().local.id, &"00".repeat(32))
        .is_err());
    assert!(p.a.connect("8.8.8.8:24810").await.is_err());
    let diagnostic = p.a.export_diagnostics();
    let c = config::load(p._a.path()).unwrap();
    assert!(!diagnostic.contains(&c.private_key));
    assert!(!diagnostic.contains(&p.a.snapshot().public_key));
}
#[tokio::test]
async fn enforces_receiver_file_limit() {
    let p = pair(true).await;
    let mut settings = p.b.snapshot().settings;
    settings.max_file_bytes = 3;
    p.b.settings(settings).unwrap();
    let path = p._a.path().join("too-big.txt");
    tokio::fs::write(&path, b"1234").await.unwrap();
    assert!(p.a.send_file(&p.b.snapshot().local.id, path).await.is_err());
    assert!(!p._b.path().join("received/too-big.txt").exists());
}

#[tokio::test]
async fn profiles_persist_and_restore_valid_arrangements() {
    let p = pair(true).await;
    let original = p.a.snapshot().layout;
    p.a.save_profile("Home desk").unwrap();
    let mut changed = original.clone();
    for d in &mut changed {
        d.rect.y += 200.0;
    }
    p.a.set_layout(changed).unwrap();
    p.a.load_profile("Home desk").unwrap();
    assert_eq!(p.a.snapshot().layout, original);
    assert!(config::load(p._a.path())
        .unwrap()
        .profiles
        .contains_key("Home desk"));
    p.a.delete_profile("Home desk").unwrap();
    assert!(p.a.load_profile("Home desk").is_err());
}

#[tokio::test]
async fn sender_can_cancel_while_waiting_for_consent() {
    let p = pair(false).await;
    let path = p._a.path().join("cancel.txt");
    tokio::fs::write(&path, b"not delivered").await.unwrap();
    let a = p.a.clone();
    let peer = p.b.snapshot().local.id;
    let task = tokio::spawn(async move { a.send_file(&peer, path).await });
    wait(|| {
        p.a.snapshot()
            .transfers
            .iter()
            .any(|t| t.state == "awaiting_acceptance")
            && p.b
                .snapshot()
                .transfers
                .iter()
                .any(|t| t.state == "offered")
    })
    .await;
    let id = p.a.snapshot().transfers[0].id.clone();
    p.a.cancel_transfer(&id).unwrap();
    assert!(tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    wait(|| p.b.snapshot().transfers[0].state != "offered").await;
    assert!(!p._b.path().join("received/cancel.txt").exists());
    assert_eq!(p.a.snapshot().transfers[0].state, "cancelled");
}

#[tokio::test]
async fn per_peer_permissions_block_input_and_file_sending() {
    let p = pair(true).await;
    let peer = p.b.snapshot().local.id;
    p.a.peer_permissions(&peer, false, false, false).unwrap();
    assert!(!p.a.capture(CapturedEvent::Move {
        x: 1919.,
        y: 500.,
        dx: 4.,
        dy: 0.
    }));
    let path = p._a.path().join("blocked.txt");
    tokio::fs::write(&path, b"blocked").await.unwrap();
    assert!(p.a.send_file(&peer, path).await.is_err());
    assert!(!p.a.snapshot().peer_permissions[&peer].input);
}

#[tokio::test]
async fn hop_shortcuts_change_focus_without_typing_the_chord() {
    let p = pair(true).await;
    let b = p.b.snapshot();
    let a = p.a.snapshot();
    p.a.set_hop_key(hop_core::HopKey {
        device_id: b.local.id.clone(),
        display_id: b.local.displays[0].id.clone(),
        keys: vec![224, 226, 30],
    })
    .unwrap();
    assert!(p
        .a
        .set_hop_key(hop_core::HopKey {
            device_id: a.local.id,
            display_id: a.local.displays[0].id.clone(),
            keys: vec![224, 226, 30]
        })
        .is_err());
    p.a.capture(CapturedEvent::Input(InputEvent::Key {
        code: 224,
        down: true,
    }));
    p.a.capture(CapturedEvent::Input(InputEvent::Key {
        code: 226,
        down: true,
    }));
    assert!(p.a.capture(CapturedEvent::Input(InputEvent::Key {
        code: 30,
        down: true
    })));
    assert_eq!(p.a.snapshot().active_device, b.local.id);
    wait(|| !p.pb.events.lock().unwrap().is_empty()).await;
    assert!(!p
        .pb
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| matches!(e, InputEvent::Key { down: true, .. })));
    for code in [30, 226, 224] {
        assert!(p
            .a
            .capture(CapturedEvent::Input(InputEvent::Key { code, down: false })));
    }
    p.a.release();
}

#[tokio::test]
async fn pausing_sender_cancels_pending_offers_and_blocks_new_files() {
    let p = pair(false).await;
    let path = p._a.path().join("pause.txt");
    tokio::fs::write(&path, b"must not arrive").await.unwrap();
    let a = p.a.clone();
    let peer = p.b.snapshot().local.id;
    let outgoing_peer = peer.clone();
    let outgoing_path = path.clone();
    let task = tokio::spawn(async move { a.send_file(&outgoing_peer, outgoing_path).await });
    wait(|| {
        p.b.snapshot()
            .transfers
            .iter()
            .any(|t| t.state == "offered")
    })
    .await;
    let mut settings = p.a.snapshot().settings;
    settings.paused = true;
    p.a.settings(settings).unwrap();
    assert!(tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    assert!(p.a.send_file(&peer, path).await.is_err());
    assert!(!p._b.path().join("received/pause.txt").exists());
    wait(|| {
        p.b.snapshot()
            .transfers
            .iter()
            .all(|t| t.state != "offered")
    })
    .await;
}

#[tokio::test]
async fn restart_restores_known_computers_and_file_send_without_rediscovery() {
    let p = pair(true).await;
    let target = p.b.snapshot();
    p.a.shutdown();
    let restarted = Service::start(p._a.path().into(), Some(0), Arc::new(Headless::default()))
        .await
        .unwrap();
    let saved = restarted
        .snapshot()
        .peers
        .into_iter()
        .find(|peer| peer.device.id == target.local.id)
        .unwrap();
    assert!(saved.trusted);
    assert_eq!(saved.device.name, target.local.name);
    assert_eq!(
        saved.public_key.as_deref(),
        Some(target.public_key.as_str())
    );
    let path = p._a.path().join("after-restart.txt");
    tokio::fs::write(&path, b"remembered address")
        .await
        .unwrap();
    let result = restarted.send_file(&target.local.id, path).await;
    restarted.shutdown();
    result.unwrap();
    assert_eq!(
        tokio::fs::read(p._b.path().join("received/after-restart.txt"))
            .await
            .unwrap(),
        b"remembered address"
    );
}
