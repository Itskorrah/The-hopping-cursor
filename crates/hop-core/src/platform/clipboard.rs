use crate::model::ClipboardPart;
use anyhow::Result;
pub fn read() -> Result<Vec<ClipboardPart>> {
    let before = super::clipboard_privacy::stamp()?;
    if before.1 {
        return Ok(vec![]);
    }
    let mut c = arboard::Clipboard::new()?;
    let mut parts = vec![];
    if let Ok(text) = c.get_text() {
        parts.push(ClipboardPart {
            mime: "text/plain".into(),
            data: text.into_bytes(),
        });
    }
    // Read HTML through the native pasteboard adapter (arboard only exposes set_html).
    #[cfg(target_os = "windows")]
    if let Some(html) = super::windows::read_html() {
        parts.push(ClipboardPart {
            mime: "text/html".into(),
            data: html.into_bytes(),
        });
    }
    #[cfg(target_os = "macos")]
    if let Some(html) = super::macos::read_html() {
        parts.push(ClipboardPart {
            mime: "text/html".into(),
            data: html.into_bytes(),
        });
    }
    if let Ok(image) = c.get_image() {
        if image.width * image.height <= 16_000_000 {
            let mut data = vec![];
            {
                let mut enc = png::Encoder::new(&mut data, image.width as u32, image.height as u32);
                enc.set_color(png::ColorType::Rgba);
                enc.set_depth(png::BitDepth::Eight);
                let mut writer = enc.write_header()?;
                writer.write_image_data(&image.bytes)?;
            }
            parts.push(ClipboardPart {
                mime: "image/png".into(),
                data,
            });
        }
    }
    // Multiple native formats are read separately. Discard a changing clipboard
    // rather than forwarding a mixture or missing a new confidentiality marker.
    if super::clipboard_privacy::stamp()? != before {
        return Ok(vec![]);
    }
    Ok(parts)
}
pub fn write(parts: &[ClipboardPart]) -> Result<()> {
    let mut c = arboard::Clipboard::new()?;
    if let Some(p) = parts.iter().find(|p| p.mime == "image/png") {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(&p.data));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info()?;
        let info = reader.info();
        anyhow::ensure!(
            (info.width as u64) * (info.height as u64) <= 16_000_000,
            "Clipboard image is too large"
        );
        let mut buf = vec![0; reader.output_buffer_size()];
        let frame = reader.next_frame(&mut buf)?;
        let bytes = &buf[..frame.buffer_size()];
        let rgba = match frame.color_type {
            png::ColorType::Rgba => bytes.to_vec(),
            png::ColorType::Rgb => bytes
                .as_chunks::<3>()
                .0
                .iter()
                .flat_map(|p| [p[0], p[1], p[2], 255])
                .collect(),
            png::ColorType::Grayscale => bytes.iter().flat_map(|v| [*v, *v, *v, 255]).collect(),
            png::ColorType::GrayscaleAlpha => bytes
                .as_chunks::<2>()
                .0
                .iter()
                .flat_map(|v| [v[0], v[0], v[0], v[1]])
                .collect(),
            _ => anyhow::bail!("Unsupported PNG format"),
        };
        c.set_image(arboard::ImageData {
            width: frame.width as usize,
            height: frame.height as usize,
            bytes: std::borrow::Cow::Owned(rgba),
        })?;
    } else if let Some(p) = parts.iter().find(|p| p.mime == "text/html") {
        let html = std::str::from_utf8(&p.data)?;
        let plain = parts
            .iter()
            .find(|p| p.mime == "text/plain")
            .and_then(|p| std::str::from_utf8(&p.data).ok());
        c.set_html(html, plain)?;
    } else if let Some(p) = parts.iter().find(|p| p.mime == "text/plain") {
        c.set_text(std::str::from_utf8(&p.data)?)?;
    }
    Ok(())
}
