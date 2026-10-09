//! Run separately on an isolated LAN: cargo test --test discovery -- --ignored
use hop_core::{platform::Headless, Service, StartOptions};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
async fn wait(mut check: impl FnMut() -> bool) {
    let start = Instant::now();
    while !check() {
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "mDNS did not resolve; check LAN multicast permissions"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
#[tokio::test]
#[ignore = "Requires LAN multicast; isolated from parallel loopback tests"]
async fn discovery_is_untrusted_and_renames_refresh_without_granting_access() {
    let da = tempfile::tempdir().unwrap();
    let db = tempfile::tempdir().unwrap();
    let options = StartOptions {
        discovery: true,
        protect_identity: false,
        loopback_only: false,
    };
    let a = Service::start_with_options(
        da.path().into(),
        Some(0),
        Arc::new(Headless::default()),
        options,
    )
    .await
    .unwrap();
    let b = Service::start_with_options(
        db.path().into(),
        Some(0),
        Arc::new(Headless::default()),
        options,
    )
    .await
    .unwrap();
    let id = b.snapshot().local.id;
    wait(|| a.snapshot().peers.iter().any(|p| p.device.id == id)).await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    let snapshot = a.snapshot();
    let peer = snapshot.peers.iter().find(|p| p.device.id == id).unwrap();
    assert!(!peer.trusted && !peer.connected && peer.public_key.is_none());
    let mut settings = b.snapshot().settings;
    settings.device_name = "Renamed test computer".into();
    b.settings(settings).unwrap();
    wait(|| {
        a.snapshot()
            .peers
            .iter()
            .any(|p| p.device.id == id && p.device.name == "Renamed test computer")
    })
    .await;
    a.shutdown();
    b.shutdown();
}
