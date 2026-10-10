//! Opt-in encrypted loopback stress/measurement fixture. No native hooks,
//! permissions, user clipboard, settings or LAN discovery are used.
use hop_core::{
    config,
    platform::{CaptureCallback, Headless, Platform},
    *,
};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

const LIMITS: [u64; 16] = [
    10,
    25,
    50,
    100,
    250,
    500,
    1000,
    2500,
    5000,
    10_000,
    25_000,
    50_000,
    100_000,
    250_000,
    1_000_000,
    u64::MAX,
];
#[derive(Default)]
struct Timing {
    buckets: [u64; 16],
    count: u64,
    sum_us: u64,
    max_us: u64,
}
impl Timing {
    fn record(&mut self, duration: Duration) {
        let us = duration.as_micros().min(u64::MAX as u128) as u64;
        self.count += 1;
        self.sum_us += us;
        self.max_us = self.max_us.max(us);
        self.buckets[LIMITS.iter().position(|bound| us <= *bound).unwrap()] += 1;
    }
    fn percentile(&self, percent: u64) -> u64 {
        let target = (self.count * percent).div_ceil(100);
        let mut seen = 0;
        for (index, count) in self.buckets.iter().enumerate() {
            seen += count;
            if seen >= target {
                return LIMITS[index];
            }
        }
        0
    }
    fn summary(&self) -> serde_json::Value {
        serde_json::json!({"count":self.count,"mean_us":self.sum_us as f64 / self.count.max(1) as f64,"max_us":self.max_us,"p50_upper_us":self.percentile(50),"p95_upper_us":self.percentile(95),"p99_upper_us":self.percentile(99),"bucket_bounds_us":LIMITS,"buckets":self.buckets})
    }
}
#[derive(Default)]
struct Probe {
    backend: Headless,
    pending: Mutex<VecDeque<Instant>>,
    timing: Mutex<Timing>,
    scrolls: AtomicU64,
}
impl Platform for Probe {
    fn name(&self) -> &str {
        "headless"
    }
    fn displays(&self) -> anyhow::Result<Vec<Display>> {
        self.backend.displays()
    }
    fn permissions(&self) -> Permissions {
        self.backend.permissions()
    }
    fn start_capture(&self, callback: CaptureCallback) -> anyhow::Result<()> {
        self.backend.start_capture(callback)
    }
    fn stop_capture(&self) {}
    fn set_forwarding(&self, forwarding: bool) -> anyhow::Result<()> {
        self.backend.set_forwarding(forwarding)
    }
    fn inject(&self, event: &InputEvent) -> anyhow::Result<()> {
        self.backend.inject(event)?;
        if matches!(event, InputEvent::Scroll { .. }) {
            let sent = self
                .pending
                .lock()
                .unwrap()
                .pop_front()
                .expect("Unscheduled synthetic wheel");
            self.timing.lock().unwrap().record(sent.elapsed());
            self.scrolls.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }
    fn read_clipboard(&self) -> anyhow::Result<Vec<ClipboardPart>> {
        self.backend.read_clipboard()
    }
    fn write_clipboard(&self, parts: &[ClipboardPart]) -> anyhow::Result<()> {
        self.backend.write_clipboard(parts)
    }
}
struct Fixture {
    a: Arc<Service>,
    b: Arc<Service>,
    pa: Arc<Probe>,
    pb: Arc<Probe>,
    da: tempfile::TempDir,
    db: tempfile::TempDir,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.a.shutdown();
        self.b.shutdown();
    }
}
async fn wait(mut check: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(8);
    while !check() {
        assert!(Instant::now() < deadline, "Stress condition timed out");
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
async fn fixture() -> Fixture {
    let da = tempfile::tempdir().unwrap();
    let db = tempfile::tempdir().unwrap();
    for dir in [&da, &db] {
        let mut c = config::load(dir.path()).unwrap();
        c.settings.auto_accept_files = true;
        c.settings.edge_delay_ms = 0;
        c.settings.clipboard_text = false;
        c.settings.clipboard_html = false;
        c.settings.clipboard_images = false;
        c.settings.receive_directory = dir.path().join("received").to_string_lossy().into();
        config::save(dir.path(), &c).unwrap();
    }
    let pa = Arc::new(Probe::default());
    let pb = Arc::new(Probe::default());
    let options = StartOptions {
        discovery: false,
        protect_identity: false,
        loopback_only: true,
    };
    let a = Service::start_with_options(da.path().into(), Some(0), pa.clone(), options)
        .await
        .unwrap();
    let b = Service::start_with_options(db.path().into(), Some(0), pb.clone(), options)
        .await
        .unwrap();
    let address = format!("127.0.0.1:{}", b.listening_port);
    assert!(a
        .connect(&address)
        .await
        .unwrap_err()
        .to_string()
        .contains("Pairing pending"));
    wait(|| b.snapshot().peers.iter().any(|p| p.public_key.is_some())).await;
    a.approve_peer(&b.snapshot().local.id, &b.snapshot().public_key)
        .unwrap();
    b.approve_peer(&a.snapshot().local.id, &a.snapshot().public_key)
        .unwrap();
    a.connect(&address).await.unwrap();
    wait(|| {
        a.snapshot().peers.iter().any(|p| p.connected)
            && b.snapshot().peers.iter().any(|p| p.connected)
    })
    .await;
    Fixture {
        a,
        b,
        pa,
        pb,
        da,
        db,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "opt-in timed loopback soak; HOP_SOAK_SECONDS=60..7200, HOP_SOAK_BULK=1"]
async fn encrypted_input_soak_reports_aggregate_latency_and_checks_recovery() {
    let seconds = std::env::var("HOP_SOAK_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60)
        .clamp(5, 7200);
    let bulk = std::env::var("HOP_SOAK_BULK").as_deref() == Ok("1");
    let f = fixture().await;
    let aid = f.a.snapshot().local.id;
    let bid = f.b.snapshot().local.id;
    let data = vec![0x5a; 1024 * 1024];
    let file = f.da.path().join("synthetic.bin");
    std::fs::write(&file, &data).unwrap();
    let mut transfers = tokio::task::JoinSet::new();
    let mut capture = Timing::default();
    let start = Instant::now();
    let mut progress = start;
    let mut cycles = 0_u64;
    let mut offered = 0_u64;
    let mut recoveries = 0_u64;
    let mut completed = 0_u64;
    let mut peak_pending = 0;
    while start.elapsed() < Duration::from_secs(seconds) {
        let (source, target, receiver, target_id) = if cycles.is_multiple_of(2) {
            (&f.a, &f.b, &f.pb, &bid)
        } else {
            (&f.b, &f.a, &f.pa, &aid)
        };
        source.jump_to_display(target_id, "headless-main").unwrap();
        wait(|| {
            target.snapshot().active_device == *target_id
                && target.snapshot().controller_device == source.snapshot().local.id
        })
        .await;
        receiver.backend.events.lock().unwrap().clear();
        let previous = receiver.scrolls.load(Ordering::SeqCst);
        for index in 0..64 {
            let begin = Instant::now();
            assert!(source.capture(CapturedEvent::Move {
                x: 0.,
                y: 0.,
                dx: if index % 2 == 0 { 0.25 } else { -0.25 },
                dy: 0.
            }));
            capture.record(begin.elapsed());
        }
        let expected = [
            InputEvent::Button {
                button: 0,
                down: true,
            },
            InputEvent::Key {
                code: 225,
                down: true,
            },
            InputEvent::Key {
                code: 8,
                down: true,
            },
            InputEvent::Scroll { x: 0., y: 0.125 },
            InputEvent::Scroll { x: -0.25, y: 0. },
            InputEvent::Scroll { x: 0.5, y: -0.5 },
            InputEvent::Scroll { x: 0., y: 1. },
            InputEvent::Key {
                code: 8,
                down: false,
            },
            InputEvent::Key {
                code: 225,
                down: false,
            },
            InputEvent::Button {
                button: 0,
                down: false,
            },
        ];
        for event in &expected {
            let begin = Instant::now();
            if matches!(event, InputEvent::Scroll { .. }) {
                receiver.pending.lock().unwrap().push_back(begin);
            }
            assert!(source.capture(CapturedEvent::Input(event.clone())));
            capture.record(begin.elapsed());
        }
        wait(|| {
            receiver.scrolls.load(Ordering::SeqCst) == previous + 4
                && receiver.backend.events.lock().unwrap().last()
                    == Some(&InputEvent::Button {
                        button: 0,
                        down: false,
                    })
        })
        .await;
        assert!(receiver.pending.lock().unwrap().is_empty());
        let discrete: Vec<_> = receiver
            .backend
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| !matches!(e, InputEvent::Move { .. } | InputEvent::ReleaseAll))
            .cloned()
            .collect();
        assert_eq!(
            discrete, expected,
            "Lost, duplicated or reordered discrete input"
        );
        for service in [&f.a, &f.b] {
            let queues = service.snapshot().control_queues;
            assert_eq!(
                queues.full_rejections, 0,
                "Unexpected input queue saturation"
            );
            peak_pending = peak_pending.max(queues.peak_pending);
        }
        source.release();
        wait(|| target.snapshot().active_device == source.snapshot().local.id).await;
        if cycles.is_multiple_of(100) && bulk && offered < 100 {
            offered += 1;
            let service = f.a.clone();
            let peer = bid.clone();
            let path = file.clone();
            transfers.spawn(async move { service.send_file(&peer, path).await });
        }
        if cycles % 200 == 199 {
            // File cancellation has separate regressions. Finish the active
            // bounded test file before measuring connection recovery here.
            while let Some(result) = transfers.join_next().await {
                result.unwrap().unwrap();
                completed += 1;
            }
            source.recover_session_gap();
            wait(|| {
                f.a.snapshot().peers.iter().any(|p| p.connected)
                    && f.b.snapshot().peers.iter().any(|p| p.connected)
            })
            .await;
            assert_eq!(source.snapshot().active_device, source.snapshot().local.id);
            recoveries += 1;
        }
        cycles += 1;
        if progress.elapsed() >= Duration::from_secs(60) {
            println!(
                "SOAK_PROGRESS seconds={} cycles={cycles} recovery_cycles={recoveries}",
                start.elapsed().as_secs()
            );
            progress = Instant::now();
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    while let Some(result) = transfers.join_next().await {
        result.unwrap().unwrap();
        completed += 1;
    }
    for transfer in f.b.snapshot().transfers {
        if transfer.state == "complete" && transfer.direction == "incoming" {
            assert_eq!(std::fs::read(transfer.saved_path.unwrap()).unwrap(), data);
        }
    }
    assert_eq!(completed, offered);
    let a_scroll = f.a.snapshot().scroll_diagnostics;
    let b_scroll = f.b.snapshot().scroll_diagnostics;
    assert_eq!(a_scroll.captured + b_scroll.captured, cycles * 4);
    assert_eq!(a_scroll.forwarded + b_scroll.forwarded, cycles * 4);
    assert_eq!(a_scroll.received + b_scroll.received, cycles * 4);
    assert_eq!(a_scroll.applied + b_scroll.applied, cycles * 4);
    assert_eq!(a_scroll.rejected + b_scroll.rejected, 0);
    let report = serde_json::json!({"scope":"synthetic encrypted loopback, in-memory receiver; excludes native OS/network/hardware latency", "seconds":start.elapsed().as_secs_f64(),"cycles":cycles,"capture_calls":capture.summary(),"a_scroll_delivery":f.pa.timing.lock().unwrap().summary(),"b_scroll_delivery":f.pb.timing.lock().unwrap().summary(),"recovery_cycles":recoveries,"verified_1mib_transfers":completed,"receiver_event_storage_bounded":true,"peak_pending_control_messages":peak_pending,"scroll_rejections":0,"a_scroll_stages":a_scroll,"b_scroll_stages":b_scroll});
    println!("SOAK_RESULT {report}");
    // Both fixture directories remain owned by TempDir and are cleaned up.
    assert!(f.db.path().exists());
}
