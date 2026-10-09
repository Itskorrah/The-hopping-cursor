use hop_core::{
    config, platform::Headless, CapturedEvent, ClipboardPart, InputEvent, LayoutDisplay, Rect,
    Service,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

async fn start_test(
    dir: std::path::PathBuf,
    port: Option<u16>,
    backend: Arc<dyn hop_core::platform::Platform>,
) -> anyhow::Result<Arc<Service>> {
    Service::start_with_options(
        dir,
        port,
        backend,
        hop_core::StartOptions {
            discovery: false,
            protect_identity: false,
            loopback_only: true,
        },
    )
    .await
}

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
    pair_with_displays(auto_accept, None).await
}

#[tokio::test]
async fn failed_capture_stays_unavailable_through_session_and_display_refresh() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(Headless::default());
    backend
        .capture_blocked
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let service = start_test(dir.path().into(), Some(0), backend.clone())
        .await
        .unwrap();
    assert!(!service.snapshot().capture_ready);
    backend
        .capture_blocked
        .store(false, std::sync::atomic::Ordering::SeqCst);
    service.refresh_session();
    service.refresh_displays().unwrap();
    assert!(!service.snapshot().capture_ready);
    assert!(service
        .snapshot()
        .diagnostics
        .iter()
        .any(|d| d.contains("Input capture:")));
    service.shutdown();
    let restarted = start_test(dir.path().into(), Some(0), backend)
        .await
        .unwrap();
    assert!(restarted.snapshot().capture_ready);
    restarted.shutdown();
}

#[tokio::test]
async fn motion_burst_over_encrypted_connection_preserves_click_and_key_order() {
    let p = pair(true).await;
    let remote = p.b.snapshot().local;
    p.a.jump_to_display(&remote.id, &remote.displays[0].id)
        .unwrap();
    wait(|| p.b.snapshot().controller_device == p.a.snapshot().local.id).await;
    p.pb.events.lock().unwrap().clear();
    for index in 0..8000 {
        assert!(p.a.capture(CapturedEvent::Move {
            x: 0.,
            y: 0.,
            dx: if index % 2 == 0 { 0.05 } else { -0.05 },
            dy: 0.
        }));
    }
    for event in [
        InputEvent::Button {
            button: 0,
            down: true,
        },
        InputEvent::Key {
            code: 8,
            down: true,
        },
        InputEvent::Key {
            code: 8,
            down: false,
        },
        InputEvent::Button {
            button: 0,
            down: false,
        },
    ] {
        assert!(p.a.capture(CapturedEvent::Input(event)));
    }
    wait(|| {
        p.pb.events
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e, InputEvent::Button { down: false, .. }))
    })
    .await;
    let events = p.pb.events.lock().unwrap();
    let discrete: Vec<_> = events
        .iter()
        .filter(|e| !matches!(e, InputEvent::Move { .. }))
        .cloned()
        .collect();
    assert_eq!(
        discrete,
        vec![
            InputEvent::Button {
                button: 0,
                down: true
            },
            InputEvent::Key {
                code: 8,
                down: true
            },
            InputEvent::Key {
                code: 8,
                down: false
            },
            InputEvent::Button {
                button: 0,
                down: false
            }
        ]
    );
    assert!(events.iter().any(|e| matches!(e, InputEvent::Move { x, y } if (*x - 960.).abs() < 0.001 && (*y - 540.).abs() < 0.001)));
    assert_eq!(p.a.snapshot().active_device, remote.id);
}
async fn pair_with_displays(auto_accept: bool, displays: Option<Vec<hop_core::Display>>) -> Pair {
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
    *pb.display_override.lock().unwrap() = displays;
    let a = start_test(da.path().to_owned(), Some(0), pa.clone())
        .await
        .unwrap();
    let b = start_test(db.path().to_owned(), Some(0), pb.clone())
        .await
        .unwrap();
    let address = format!("127.0.0.1:{}", b.listening_port);
    let introduction = a.connect(&address).await.unwrap_err();
    assert!(
        introduction.to_string().contains("Pairing pending"),
        "Expected pairing introduction, received: {introduction:#}"
    );
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

fn pc_displays() -> Vec<hop_core::Display> {
    vec![
        hop_core::Display {
            id: "right".into(),
            name: "Right".into(),
            bounds: Rect {
                x: 1920.,
                y: 0.,
                width: 1920.,
                height: 1080.,
            },
            scale: 1.,
            primary: false,
        },
        hop_core::Display {
            id: "primary".into(),
            name: "Middle".into(),
            bounds: Rect {
                x: 0.,
                y: 0.,
                width: 1920.,
                height: 1080.,
            },
            scale: 1.,
            primary: true,
        },
    ]
}

#[tokio::test]
async fn pc_primary_left_edge_routes_to_mac_with_exclusive_keyboard_and_buttons() {
    let p = pair_with_displays(true, Some(pc_displays())).await;
    let a = p.a.snapshot().local;
    let b = p.b.snapshot().local;
    let layout = vec![
        LayoutDisplay {
            device_id: a.id.clone(),
            display_id: a.displays[0].id.clone(),
            rect: Rect {
                x: 0.,
                y: 0.,
                width: 1920.,
                height: 1080.,
            },
            hidden: false,
        },
        LayoutDisplay {
            device_id: b.id.clone(),
            display_id: "primary".into(),
            rect: Rect {
                x: 1920.,
                y: 0.,
                width: 1920.,
                height: 1080.,
            },
            hidden: false,
        },
        LayoutDisplay {
            device_id: b.id.clone(),
            display_id: "right".into(),
            rect: Rect {
                x: 3840.,
                y: 0.,
                width: 1920.,
                height: 1080.,
            },
            hidden: false,
        },
    ];
    p.a.set_layout(layout.clone()).unwrap();
    p.b.set_layout(layout).unwrap();
    assert!(p.b.capture(CapturedEvent::Move {
        x: 0.,
        y: 500.,
        dx: -5.,
        dy: 0.
    }));
    for event in [
        InputEvent::Button {
            button: 1,
            down: true,
        },
        InputEvent::Button {
            button: 1,
            down: false,
        },
        InputEvent::Key {
            code: 4,
            down: true,
        },
        InputEvent::Key {
            code: 4,
            down: false,
        },
    ] {
        assert!(p.b.capture(CapturedEvent::Input(event)));
    }
    wait(|| {
        p.pa.events.lock().unwrap().contains(&InputEvent::Key {
            code: 4,
            down: false,
        })
    })
    .await;
    assert_eq!(p.b.snapshot().active_device, a.id);
    assert!(p.pb.forwarding.load(std::sync::atomic::Ordering::SeqCst));
    assert!(!p.pb.events.lock().unwrap().contains(&InputEvent::Button {
        button: 1,
        down: true
    }));
    assert!(!p.pb.events.lock().unwrap().contains(&InputEvent::Key {
        code: 4,
        down: true
    }));
    p.b.release();
    wait(|| {
        p.pa.events
            .lock()
            .unwrap()
            .contains(&InputEvent::ReleaseAll)
    })
    .await;
    assert!(!p.pb.forwarding.load(std::sync::atomic::Ordering::SeqCst));
}

#[tokio::test]
async fn native_local_monitors_are_not_warped_by_a_reordered_workspace() {
    let p = pair_with_displays(true, Some(pc_displays())).await;
    let id = p.b.snapshot().local.id;
    p.b.set_layout(vec![
        LayoutDisplay {
            device_id: id.clone(),
            display_id: "right".into(),
            rect: Rect {
                x: 0.,
                y: 0.,
                width: 1920.,
                height: 1080.,
            },
            hidden: false,
        },
        LayoutDisplay {
            device_id: id,
            display_id: "primary".into(),
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
    p.pb.events.lock().unwrap().clear();
    assert!(!p.b.capture(CapturedEvent::Move {
        x: 0.,
        y: 500.,
        dx: -5.,
        dy: 0.
    }));
    assert!(
        p.pb.events.lock().unwrap().is_empty(),
        "Windows must retain its own native monitor transitions"
    );
}

#[tokio::test]
async fn remote_drag_across_two_monitors_keeps_the_same_input_lease() {
    let p = pair_with_displays(true, Some(pc_displays())).await;
    let a = p.a.snapshot().local;
    let b = p.b.snapshot().local;
    p.a.set_layout(vec![
        LayoutDisplay {
            device_id: a.id.clone(),
            display_id: a.displays[0].id.clone(),
            rect: Rect {
                x: 0.,
                y: 0.,
                width: 1920.,
                height: 1080.,
            },
            hidden: false,
        },
        LayoutDisplay {
            device_id: b.id.clone(),
            display_id: "primary".into(),
            rect: Rect {
                x: 1920.,
                y: 0.,
                width: 1920.,
                height: 1080.,
            },
            hidden: false,
        },
        LayoutDisplay {
            device_id: b.id.clone(),
            display_id: "right".into(),
            rect: Rect {
                x: 3840.,
                y: 0.,
                width: 1920.,
                height: 1080.,
            },
            hidden: false,
        },
    ])
    .unwrap();
    p.a.jump_to_display(&b.id, "primary").unwrap();
    assert!(p.pa.forwarding.load(std::sync::atomic::Ordering::SeqCst));
    assert!(p.a.capture(CapturedEvent::Unsupported));
    assert!(p.a.capture(CapturedEvent::Input(InputEvent::Button {
        button: 0,
        down: true
    })));
    wait(|| {
        p.pb.events.lock().unwrap().contains(&InputEvent::Button {
            button: 0,
            down: true,
        })
    })
    .await;
    p.pb.events.lock().unwrap().clear();
    assert!(p.a.capture(CapturedEvent::Move {
        x: 960.,
        y: 540.,
        dx: 1000.,
        dy: 0.
    }));
    wait(|| {
        p.pb.events
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e, InputEvent::Move { x, .. } if *x > 1920.))
    })
    .await;
    let events = p.pb.events.lock().unwrap().clone();
    assert!(
        !events.contains(&InputEvent::ReleaseAll),
        "Crossing monitors on the same PC must preserve a held drag"
    );
    assert!(
        !events.contains(&InputEvent::Button {
            button: 0,
            down: true
        }),
        "A monitor change must not synthesize a second mouse-down"
    );
    assert_eq!(p.a.snapshot().active_device, b.id);
    p.a.release();
    assert!(!p.pa.forwarding.load(std::sync::atomic::Ordering::SeqCst));
    assert!(!p.a.capture(CapturedEvent::Unsupported));
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
    assert!(!p.pa.forwarding.load(std::sync::atomic::Ordering::SeqCst));
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
    let restarted = start_test(p._a.path().into(), Some(0), Arc::new(Headless::default()))
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

#[tokio::test]
async fn simultaneous_connections_keep_one_live_control_channel() {
    let p = pair(true).await;
    p.a.shutdown();
    p.b.shutdown();
    tokio::time::sleep(Duration::from_millis(80)).await;
    let a = start_test(p._a.path().into(), Some(0), p.pa.clone())
        .await
        .unwrap();
    let b = start_test(p._b.path().into(), Some(0), p.pb.clone())
        .await
        .unwrap();
    let a_address = format!("127.0.0.1:{}", a.listening_port);
    let b_address = format!("127.0.0.1:{}", b.listening_port);
    let (one, two) = tokio::join!(a.connect(&b_address), b.connect(&a_address));
    one.unwrap();
    two.unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(a
        .snapshot()
        .peers
        .iter()
        .any(|p| p.connected && p.latency_ms.is_some()));
    assert!(b
        .snapshot()
        .peers
        .iter()
        .any(|p| p.connected && p.latency_ms.is_some()));
    let saved_b = config::load(p._b.path()).unwrap();
    assert_eq!(
        saved_b.peers[&a.snapshot().local.id].address.as_deref(),
        Some(a_address.as_str())
    );
    let file = p._a.path().join("after-race.txt");
    tokio::fs::write(&file, b"connection survived")
        .await
        .unwrap();
    a.send_file(&b.snapshot().local.id, file).await.unwrap();
    a.shutdown();
    b.shutdown();
}

#[tokio::test]
async fn hotplug_releases_input_reconciles_profiles_and_refreshes_peer_metadata() {
    let p = pair(true).await;
    let original = p.a.snapshot();
    p.a.save_profile("Desk").unwrap();
    p.a.jump_to_display(&p.b.snapshot().local.id, "headless-main")
        .unwrap();
    wait(|| {
        p.b.snapshot().active_device != p.b.snapshot().local.id
            || !p.pb.events.lock().unwrap().is_empty()
    })
    .await;
    let mut display = original.local.displays[0].clone();
    display.id = "new-monitor".into();
    display.bounds.width = 2560.;
    *p.pa.display_override.lock().unwrap() = Some(vec![display]);
    assert!(p.a.refresh_displays().unwrap());
    assert_eq!(p.a.snapshot().active_device, original.local.id);
    assert!(!p
        .a
        .snapshot()
        .layout
        .iter()
        .any(|d| d.device_id == original.local.id && d.display_id == "headless-main"));
    p.a.load_profile("Desk").unwrap();
    assert!(p
        .a
        .snapshot()
        .layout
        .iter()
        .any(|d| d.device_id == original.local.id && d.display_id == "new-monitor"));
    assert_eq!(
        config::load(p._a.path()).unwrap().profiles["Desk"],
        original.layout
    );
    wait(|| {
        p.a.snapshot().peers.iter().any(|peer| peer.connected)
            && p.b.snapshot().peers.iter().any(|peer| {
                peer.connected && peer.device.displays.iter().any(|d| d.id == "new-monitor")
            })
    })
    .await;
    assert!(p
        .pb
        .events
        .lock()
        .unwrap()
        .contains(&InputEvent::ReleaseAll));
}

#[tokio::test]
async fn losing_all_displays_releases_input_without_erasing_saved_layout() {
    let p = pair(true).await;
    let before = p.a.snapshot();
    let peer = p.b.snapshot().local.id;
    p.a.jump_to_display(&peer, "headless-main").unwrap();
    *p.pa.display_override.lock().unwrap() = Some(vec![]);
    assert!(p.a.refresh_displays().is_err());
    p.a.refresh_session(); // Permission polling must not re-enable missing displays.
    assert_eq!(p.a.snapshot().active_device, before.local.id);
    assert_eq!(p.a.snapshot().layout, before.layout);
    assert!(p.a.jump_to_display(&peer, "headless-main").is_err());
    *p.pa.display_override.lock().unwrap() = None;
    assert!(!p.a.refresh_displays().unwrap());
    wait(|| p.a.snapshot().peers.iter().any(|peer| peer.connected)).await;
    p.a.jump_to_display(&peer, "headless-main").unwrap();
}

#[tokio::test]
async fn shutdown_does_not_allow_manual_reconnection_or_leave_connected_status() {
    let p = pair(true).await;
    p.a.shutdown();
    assert!(p.a.snapshot().peers.iter().all(|peer| !peer.connected));
    assert!(p
        .a
        .connect(&format!("127.0.0.1:{}", p.b.listening_port))
        .await
        .is_err());
    assert!(!p.a.capture(CapturedEvent::Input(InputEvent::Key {
        code: 4,
        down: true
    })));
}

#[tokio::test]
async fn duplicate_incoming_offer_cannot_replace_pending_consent() {
    use hop_core::{crypto::SecureConnection, BlobKind, Message};
    let p = pair(false).await;
    let path = p._a.path().join("original.txt");
    tokio::fs::write(&path, b"original bytes").await.unwrap();
    let source = p.a.clone();
    let peer = p.b.snapshot().local.id;
    let sending = tokio::spawn(async move { source.send_file(&peer, path).await });
    wait(|| {
        p.b.snapshot()
            .transfers
            .iter()
            .any(|t| t.state == "offered")
    })
    .await;
    let original = p.b.snapshot().transfers[0].clone();
    let identity = config::Identity::from_config(&config::load(p._a.path()).unwrap()).unwrap();
    let stream = tokio::net::TcpStream::connect(("127.0.0.1", p.b.listening_port))
        .await
        .unwrap();
    let connection = SecureConnection::handshake(stream, &identity, true)
        .await
        .unwrap();
    let (mut reader, mut writer) = connection.split();
    writer
        .send(&Message::Hello {
            version: 2,
            device: p.a.snapshot().local,
            purpose: "data".into(),
            port: p.a.listening_port,
            approved: true,
        })
        .await
        .unwrap();
    assert!(matches!(
        reader.recv().await.unwrap(),
        Message::Hello { approved: true, .. }
    ));
    writer
        .send(&Message::BlobStart {
            id: original.id.clone(),
            kind: BlobKind::File,
            name: "forged.txt".into(),
            size: 0,
            hash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
        })
        .await
        .unwrap();
    let rejected = tokio::time::timeout(Duration::from_secs(2), reader.recv())
        .await
        .expect("Duplicate offer must close without waiting for consent");
    assert!(rejected.is_err());
    let pending =
        p.b.snapshot()
            .transfers
            .into_iter()
            .find(|t| t.id == original.id)
            .unwrap();
    assert_eq!(pending.name, "original.txt");
    assert_eq!(pending.state, "offered");
    p.b.respond_transfer(&original.id, true).unwrap();
    sending.await.unwrap().unwrap();
    assert_eq!(
        tokio::fs::read(p._b.path().join("received/original.txt"))
            .await
            .unwrap(),
        b"original bytes"
    );
}

#[tokio::test]
async fn copied_files_use_consent_and_only_verified_received_files_are_publishable() {
    let p = pair(false).await;
    let path = p._a.path().join("copied.txt");
    tokio::fs::write(&path, b"clipboard file test")
        .await
        .unwrap();
    *p.pa.file_clipboard.lock().unwrap() = vec![path.clone(), path];
    let source = p.a.clone();
    let peer = p.b.snapshot().local.id;
    let sending = tokio::spawn(async move { source.send_copied_files(&peer).await });
    wait(|| {
        p.b.snapshot()
            .transfers
            .iter()
            .any(|t| t.state == "offered")
    })
    .await;
    let id = p.b.snapshot().transfers[0].id.clone();
    assert!(p.b.copy_received_file(&id).is_err());
    assert!(p.pb.file_clipboard.lock().unwrap().is_empty());
    p.b.respond_transfer(&id, true).unwrap();
    sending.await.unwrap().unwrap();
    p.b.copy_received_file(&id).unwrap();
    assert_eq!(
        *p.pb.file_clipboard.lock().unwrap(),
        vec![p
            ._b
            .path()
            .join("received/copied.txt")
            .canonicalize()
            .unwrap()]
    );
    assert!(p.a.copy_received_file(&id).is_err());
    assert_eq!(
        p.b.snapshot().transfers.len(),
        1,
        "Duplicate source clipboard entries must not send twice"
    );
    let peer = p.b.snapshot().local.id;
    p.a.peer_permissions(&peer, true, false, true).unwrap();
    assert!(p.a.send_copied_files(&peer).await.is_err());
}

#[tokio::test]
async fn bulk_pressure_is_bounded_and_control_heartbeats_continue() {
    let p = pair(false).await;
    let path = p._a.path().join("pressure.txt");
    tokio::fs::write(&path, b"bounded bulk").await.unwrap();
    let peer = p.b.snapshot().local.id;
    let mut jobs = Vec::new();
    for _ in 0..4 {
        let source = p.a.clone();
        let peer = peer.clone();
        let path = path.clone();
        jobs.push(tokio::spawn(
            async move { source.send_file(&peer, path).await },
        ));
    }
    wait(|| {
        p.b.snapshot()
            .transfers
            .iter()
            .filter(|t| t.state == "offered")
            .count()
            == 4
    })
    .await;
    assert!(p
        .a
        .send_file(&peer, path)
        .await
        .unwrap_err()
        .to_string()
        .contains("Four bulk transfers"));
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(p
        .a
        .snapshot()
        .peers
        .iter()
        .any(|peer| peer.connected && peer.latency_ms.is_some()));
    p.a.release();
    for transfer in p.b.snapshot().transfers {
        p.b.respond_transfer(&transfer.id, false).unwrap();
    }
    for job in jobs {
        assert!(job.await.unwrap().is_err());
    }
    assert!(p.a.snapshot().peers.iter().any(|peer| peer.connected));
}

#[tokio::test]
async fn protected_session_releases_input_cancels_offers_and_recovers() {
    use std::sync::atomic::Ordering;
    let p = pair(false).await;
    let peer = p.b.snapshot().local.id;
    p.a.jump_to_display(&peer, "headless-main").unwrap();
    let path = p._a.path().join("session.txt");
    tokio::fs::write(&path, b"session transition")
        .await
        .unwrap();
    let source = p.a.clone();
    let id = peer.clone();
    let file = path.clone();
    let task = tokio::spawn(async move { source.send_file(&id, file).await });
    wait(|| {
        p.b.snapshot()
            .transfers
            .iter()
            .any(|t| t.state == "offered")
    })
    .await;
    p.pa.session_blocked.store(true, Ordering::SeqCst);
    p.a.refresh_session();
    assert!(!p.a.snapshot().session_available);
    assert_eq!(p.a.snapshot().active_device, p.a.snapshot().local.id);
    assert!(task.await.unwrap().is_err());
    assert!(p.a.send_file(&peer, path).await.is_err());
    assert!(p.a.jump_to_display(&peer, "headless-main").is_err());
    p.pa.session_blocked.store(false, Ordering::SeqCst);
    p.a.refresh_session();
    wait(|| {
        p.a.snapshot().peers.iter().any(|p| p.connected)
            && p.b.snapshot().peers.iter().any(|p| p.connected)
    })
    .await;
    assert!(p
        .pb
        .events
        .lock()
        .unwrap()
        .contains(&InputEvent::ReleaseAll));
}

#[tokio::test]
async fn wake_recovery_clears_ownership_and_pressed_modifier_state() {
    let p = pair(true).await;
    p.a.capture(CapturedEvent::Input(InputEvent::Key {
        code: 224,
        down: true,
    }));
    p.a.jump_to_display(&p.b.snapshot().local.id, "headless-main")
        .unwrap();
    wait(|| {
        p.pb.events.lock().unwrap().contains(&InputEvent::Key {
            code: 224,
            down: true,
        })
    })
    .await;
    p.pa.events.lock().unwrap().clear();
    p.a.recover_session_gap();
    assert_eq!(p.a.snapshot().active_device, p.a.snapshot().local.id);
    assert!(
        !p.a.capture(CapturedEvent::Input(InputEvent::Key {
            code: 41,
            down: true
        })),
        "Stale Ctrl state must not turn a later Escape into a recovery chord"
    );
    assert!(
        !p.pa.events.lock().unwrap().contains(&InputEvent::Key {
            code: 224,
            down: true
        }),
        "Recovery must not restore a stale physical modifier"
    );
    wait(|| {
        p.pb.events
            .lock()
            .unwrap()
            .contains(&InputEvent::ReleaseAll)
    })
    .await;
}

#[tokio::test]
async fn shared_arrangement_requires_acceptance_and_routes_both_directions() {
    let p = pair_with_displays(true, Some(pc_displays())).await;
    let layout = p.a.snapshot().layout;
    assert_ne!(p.b.snapshot().layout, layout);
    assert_eq!(p.a.share_layout().unwrap(), 1);
    wait(|| !p.b.snapshot().layout_offers.is_empty()).await;
    assert_ne!(
        p.b.snapshot().layout,
        layout,
        "A peer must not silently overwrite local work"
    );
    p.b.accept_layout(&p.a.snapshot().local.id).unwrap();
    assert_eq!(p.b.snapshot().layout, layout);
    let a = p.a.snapshot().local;
    let b = p.b.snapshot().local;
    p.b.jump_to_display(&a.id, &a.displays[0].id).unwrap();
    wait(|| {
        p.pa.events
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e, InputEvent::Move { .. }))
    })
    .await;
    assert!(p.pb.forwarding.load(std::sync::atomic::Ordering::SeqCst));
    assert!(p.b.capture(CapturedEvent::Input(InputEvent::Key {
        code: 4,
        down: true
    })));
    wait(|| {
        p.pa.events.lock().unwrap().contains(&InputEvent::Key {
            code: 4,
            down: true,
        })
    })
    .await;
    p.b.release();
    wait(|| p.pa.forwarding.load(std::sync::atomic::Ordering::SeqCst)).await;
    assert!(!p.pb.forwarding.load(std::sync::atomic::Ordering::SeqCst));
    // Reclaim the other physical mouse, then use its keyboard on the PC.
    p.a.capture(CapturedEvent::Move {
        x: 100.,
        y: 100.,
        dx: 2.,
        dy: 0.,
    });
    p.a.jump_to_display(&b.id, "primary").unwrap();
    wait(|| !p.pb.forwarding.load(std::sync::atomic::Ordering::SeqCst)).await;
    assert!(p.a.capture(CapturedEvent::Input(InputEvent::Key {
        code: 5,
        down: true
    })));
    wait(|| {
        p.pb.events.lock().unwrap().contains(&InputEvent::Key {
            code: 5,
            down: true,
        })
    })
    .await;
}

#[tokio::test]
async fn departed_cursor_hides_idle_peer_and_its_keyboard_follows_focus() {
    let p = pair(true).await;
    let a = p.a.snapshot().local;
    let b = p.b.snapshot().local;
    p.a.jump_to_display(&b.id, &b.displays[0].id).unwrap();
    wait(|| p.pa.forwarding.load(std::sync::atomic::Ordering::SeqCst)).await;
    p.a.release();
    wait(|| p.pb.forwarding.load(std::sync::atomic::Ordering::SeqCst)).await;
    p.pa.events.lock().unwrap().clear();
    assert!(p.b.capture(CapturedEvent::Input(InputEvent::Key {
        code: 6,
        down: true
    })));
    wait(|| {
        p.pa.events.lock().unwrap().contains(&InputEvent::Key {
            code: 6,
            down: true,
        })
    })
    .await;
    assert_eq!(p.b.snapshot().active_device, a.id);
    assert!(p.b.capture(CapturedEvent::Input(InputEvent::Key {
        code: 6,
        down: false
    })));
    wait(|| {
        p.pa.events.lock().unwrap().contains(&InputEvent::Key {
            code: 6,
            down: false,
        })
    })
    .await;
    p.b.capture(CapturedEvent::Move {
        x: 100.,
        y: 100.,
        dx: 1.,
        dy: 0.,
    });
    wait(|| p.pa.forwarding.load(std::sync::atomic::Ordering::SeqCst)).await;
    assert!(!p.pb.forwarding.load(std::sync::atomic::Ordering::SeqCst));
    p.a.shutdown();
    wait(|| !p.pb.forwarding.load(std::sync::atomic::Ordering::SeqCst)).await;
}

#[tokio::test]
async fn automatic_file_clipboard_publishes_verified_batch_without_echo() {
    let p = pair(true).await;
    for s in [&p.a, &p.b] {
        let mut settings = s.snapshot().settings;
        settings.clipboard_files = true;
        s.settings(settings).unwrap();
    }
    let paths = vec![p._a.path().join("one.txt"), p._a.path().join("two.txt")];
    for path in &paths {
        tokio::fs::write(path, b"verified copied file")
            .await
            .unwrap();
    }
    *p.pa.file_clipboard.lock().unwrap() = paths;
    wait(|| p.pb.file_clipboard.lock().unwrap().len() == 2).await;
    let copied = p.pb.file_clipboard.lock().unwrap().clone();
    for path in copied {
        assert_eq!(
            tokio::fs::read(path).await.unwrap(),
            b"verified copied file"
        );
    }
    tokio::time::sleep(Duration::from_millis(1600)).await;
    assert_eq!(
        p.b.snapshot().transfers.len(),
        2,
        "Received files must not bounce back"
    );
    assert!(p
        .a
        .snapshot()
        .transfers
        .iter()
        .all(|t| t.direction == "outgoing"));
}

#[tokio::test]
async fn file_clipboard_consent_preserves_a_newer_recipient_clipboard() {
    let p = pair(false).await;
    for s in [&p.a, &p.b] {
        let mut settings = s.snapshot().settings;
        settings.clipboard_files = true;
        s.settings(settings).unwrap();
    }
    let path = p._a.path().join("copied.txt");
    tokio::fs::write(&path, b"safe").await.unwrap();
    *p.pa.file_clipboard.lock().unwrap() = vec![path];
    wait(|| {
        p.b.snapshot()
            .transfers
            .iter()
            .any(|t| t.state == "offered")
    })
    .await;
    let newer = p._b.path().join("newer.txt");
    tokio::fs::write(&newer, b"new choice").await.unwrap();
    *p.pb.file_clipboard.lock().unwrap() = vec![newer.clone()];
    let id =
        p.b.snapshot()
            .transfers
            .iter()
            .find(|t| t.state == "offered")
            .unwrap()
            .id
            .clone();
    p.b.respond_transfer(&id, true).unwrap();
    wait(|| {
        p.b.snapshot()
            .transfers
            .iter()
            .any(|t| t.id == id && t.state == "complete")
    })
    .await;
    assert_eq!(*p.pb.file_clipboard.lock().unwrap(), vec![newer]);
}

#[tokio::test]
async fn physical_controller_takeover_does_not_echo_leave_to_new_owner() {
    let p = pair(true).await;
    let a = p.a.snapshot().local;
    let b = p.b.snapshot().local;
    p.a.jump_to_display(&b.id, &b.displays[0].id).unwrap();
    wait(|| p.pa.forwarding.load(std::sync::atomic::Ordering::SeqCst)).await;
    wait(|| {
        p.pb.events
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e, InputEvent::Move { .. }))
    })
    .await;
    p.b.capture(CapturedEvent::Move {
        x: 100.,
        y: 100.,
        dx: 1.,
        dy: 0.,
    });
    wait(|| p.a.snapshot().controller_device == b.id && p.a.snapshot().active_device == b.id).await;
    p.b.jump_to_display(&a.id, &a.displays[0].id).unwrap();
    wait(|| {
        p.b.snapshot().active_device == a.id
            && !p.pa.forwarding.load(std::sync::atomic::Ordering::SeqCst)
    })
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    p.pa.events.lock().unwrap().clear();
    assert!(p.b.capture(CapturedEvent::Input(InputEvent::Key {
        code: 7,
        down: true
    })));
    wait(|| {
        p.pa.events.lock().unwrap().contains(&InputEvent::Key {
            code: 7,
            down: true,
        })
    })
    .await;
    // Physical keys on the destination must not cancel the inbound mouse lease.
    assert!(!p.a.capture(CapturedEvent::Input(InputEvent::Key {
        code: 8,
        down: true
    })));
    assert!(p.b.capture(CapturedEvent::Input(InputEvent::Key {
        code: 9,
        down: true
    })));
    wait(|| {
        p.pa.events.lock().unwrap().contains(&InputEvent::Key {
            code: 9,
            down: true,
        })
    })
    .await;
}

#[tokio::test]
async fn shared_keyboard_disconnect_releases_synthetic_keys_and_emergency_claims_cursor() {
    let p = pair(true).await;
    let a = p.a.snapshot().local;
    let b = p.b.snapshot().local;
    p.a.jump_to_display(&b.id, &b.displays[0].id).unwrap();
    wait(|| p.pa.forwarding.load(std::sync::atomic::Ordering::SeqCst)).await;
    p.a.release();
    wait(|| p.pb.forwarding.load(std::sync::atomic::Ordering::SeqCst)).await;
    assert!(p.b.capture(CapturedEvent::Input(InputEvent::Key {
        code: 224,
        down: true
    })));
    wait(|| {
        p.pa.events.lock().unwrap().contains(&InputEvent::Key {
            code: 224,
            down: true,
        })
    })
    .await;
    p.b.release();
    wait(|| p.pa.forwarding.load(std::sync::atomic::Ordering::SeqCst)).await;
    assert!(!p.pb.forwarding.load(std::sync::atomic::Ordering::SeqCst));
    // Return focus to a, then hold a key using b's idle keyboard and disconnect.
    p.a.capture(CapturedEvent::Move {
        x: 100.,
        y: 100.,
        dx: 1.,
        dy: 0.,
    });
    wait(|| p.pb.forwarding.load(std::sync::atomic::Ordering::SeqCst)).await;
    p.pa.events.lock().unwrap().clear();
    assert!(p.b.capture(CapturedEvent::Input(InputEvent::Key {
        code: 225,
        down: true
    })));
    wait(|| {
        p.pa.events.lock().unwrap().contains(&InputEvent::Key {
            code: 225,
            down: true,
        })
    })
    .await;
    p.pa.events.lock().unwrap().clear();
    p.b.shutdown();
    wait(|| {
        p.pa.events
            .lock()
            .unwrap()
            .contains(&InputEvent::ReleaseAll)
    })
    .await;
    assert!(!p.pa.forwarding.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(p.a.snapshot().active_device, a.id);
}

#[tokio::test]
async fn later_file_copy_wins_when_consent_is_answered_out_of_order() {
    let p = pair(false).await;
    for service in [&p.a, &p.b] {
        let mut settings = service.snapshot().settings;
        settings.clipboard_files = true;
        service.settings(settings).unwrap();
    }
    let old = p._a.path().join("old.txt");
    let new = p._a.path().join("new.txt");
    tokio::fs::write(&old, b"old").await.unwrap();
    tokio::fs::write(&new, b"new").await.unwrap();
    *p.pa.file_clipboard.lock().unwrap() = vec![old];
    wait(|| {
        p.b.snapshot()
            .transfers
            .iter()
            .any(|t| t.name == "old.txt" && t.state == "offered")
    })
    .await;
    *p.pa.file_clipboard.lock().unwrap() = vec![new];
    wait(|| {
        p.b.snapshot()
            .transfers
            .iter()
            .any(|t| t.name == "new.txt" && t.state == "offered")
    })
    .await;
    let transfers = p.b.snapshot().transfers;
    let newest = transfers.iter().find(|t| t.name == "new.txt").unwrap();
    p.b.respond_transfer(&newest.id, true).unwrap();
    wait(|| {
        p.pb.file_clipboard
            .lock()
            .unwrap()
            .iter()
            .any(|p| p.file_name().unwrap() == "new.txt")
    })
    .await;
    let oldest = transfers.iter().find(|t| t.name == "old.txt").unwrap();
    p.b.respond_transfer(&oldest.id, true).unwrap();
    wait(|| {
        p.b.snapshot()
            .transfers
            .iter()
            .any(|t| t.id == oldest.id && t.state == "complete")
    })
    .await;
    assert_eq!(p.pb.file_clipboard.lock().unwrap().len(), 1);
    assert_eq!(
        p.pb.file_clipboard.lock().unwrap()[0].file_name().unwrap(),
        "new.txt"
    );
}

#[tokio::test]
async fn unavailable_file_clipboard_preserves_verified_files_and_reports_fallback() {
    let p = pair(true).await;
    for service in [&p.a, &p.b] {
        let mut settings = service.snapshot().settings;
        settings.clipboard_files = true;
        service.settings(settings).unwrap();
    }
    p.pb.file_clipboard_blocked
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let path = p._a.path().join("clipboard-fallback.txt");
    tokio::fs::write(&path, b"verified fallback").await.unwrap();
    *p.pa.file_clipboard.lock().unwrap() = vec![path];
    wait(|| {
        p.b.snapshot()
            .diagnostics
            .iter()
            .any(|message| message.contains("system file clipboard could not be updated"))
    })
    .await;
    wait(|| {
        p.a.snapshot()
            .transfers
            .iter()
            .any(|transfer| transfer.state == "complete")
    })
    .await;
    let received =
        p.b.snapshot()
            .transfers
            .into_iter()
            .find(|transfer| transfer.direction == "incoming")
            .unwrap();
    assert_eq!(received.state, "complete");
    assert_eq!(
        tokio::fs::read(received.saved_path.unwrap()).await.unwrap(),
        b"verified fallback"
    );
    assert!(p.pb.file_clipboard.lock().unwrap().is_empty());
}
