//! Synthetic, public protocol examples. Never seed fuzzing with user data.
use hop_core::model::{BlobKind, Device, Display, InputEvent, Message, Rect};
fn main() -> std::io::Result<()> {
    let display = Display {
        id: "synthetic-display".into(),
        name: "Test display".into(),
        bounds: Rect {
            x: 0.,
            y: 0.,
            width: 1920.,
            height: 1080.,
        },
        scale: 1.,
        primary: true,
    };
    let messages = [
        Message::Hello {
            version: 1,
            device: Device {
                id: "a".repeat(32),
                name: "Synthetic".into(),
                platform: "headless".into(),
                displays: vec![display],
            },
            purpose: "control".into(),
            port: 24810,
            approved: false,
        },
        Message::Ping(1),
        Message::Pong(u64::MAX),
        Message::Enter {
            display_id: "synthetic-display".into(),
            x: 960.,
            y: 540.,
        },
        Message::Leave,
        Message::Input(InputEvent::Key {
            code: 4,
            down: true,
        }),
        Message::BlobStart {
            id: "synthetic".into(),
            kind: BlobKind::File,
            name: "test.txt".into(),
            size: 3,
            hash: "0".repeat(64),
        },
        Message::BlobAccept {
            id: "synthetic".into(),
            accepted: true,
        },
        Message::BlobChunk {
            id: "synthetic".into(),
            offset: 0,
            data: vec![0; 32768],
        },
        Message::BlobEnd {
            id: "synthetic".into(),
        },
        Message::BlobResult {
            id: "synthetic".into(),
            ok: false,
            error: Some("synthetic error".into()),
        },
    ];
    std::fs::create_dir_all("fuzz/corpus/control_frames")?;
    std::fs::write(
        "fuzz/corpus/control_frames/malformed-display-count",
        include_bytes!("../../crates/hop-core/tests/fixtures/malformed-display-count.bin"),
    )?;
    for (index, message) in messages.iter().enumerate() {
        std::fs::write(
            format!("fuzz/corpus/control_frames/seed-{index}"),
            postcard::to_allocvec(message).unwrap(),
        )?;
    }
    Ok(())
}
