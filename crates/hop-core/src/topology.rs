use crate::model::{LayoutDisplay, Rect};
use anyhow::Result;

pub fn validate(layout: &[LayoutDisplay]) -> Result<()> {
    anyhow::ensure!(layout.len() <= 64, "At most 64 displays are supported");
    for (i, a) in layout.iter().enumerate() {
        anyhow::ensure!(a.rect.valid(), "Invalid display dimensions");
        anyhow::ensure!(
            !a.device_id.is_empty() && !a.display_id.is_empty(),
            "Display identity is required"
        );
        for b in &layout[i + 1..] {
            anyhow::ensure!(
                a.device_id != b.device_id || a.display_id != b.display_id,
                "Duplicate display"
            );
            if !a.hidden && !b.hidden {
                anyhow::ensure!(
                    !overlap(&a.rect, &b.rect),
                    "Visible displays cannot overlap; drag them edge to edge"
                );
            }
        }
    }
    anyhow::ensure!(
        layout.iter().any(|d| !d.hidden),
        "Keep at least one display visible"
    );
    Ok(())
}
/// Reconcile a freshly authenticated display list. Saved profiles keep their
/// original contents; the active layout contains only currently attached screens.
pub fn reconcile_device(layout: &mut Vec<LayoutDisplay>, device: &crate::model::Device) {
    layout.retain(|entry| {
        entry.device_id != device.id || device.displays.iter().any(|d| d.id == entry.display_id)
    });
    let right = layout
        .iter()
        .filter(|d| !d.hidden)
        .map(|d| d.rect.x + d.rect.width)
        .fold(0., f64::max);
    let min_x = device
        .displays
        .iter()
        .map(|d| d.bounds.x)
        .fold(f64::INFINITY, f64::min);
    let min_y = device
        .displays
        .iter()
        .map(|d| d.bounds.y)
        .fold(f64::INFINITY, f64::min);
    // Enumeration order is not physical order. Retain native monitor offsets.
    let anchor = layout
        .iter()
        .find_map(|entry| {
            if entry.device_id != device.id {
                return None;
            }
            let native = device.displays.iter().find(|d| d.id == entry.display_id)?;
            Some((
                entry.rect.x - native.bounds.x,
                entry.rect.y - native.bounds.y,
            ))
        })
        .unwrap_or((right - min_x, -min_y));
    for display in &device.displays {
        if !layout
            .iter()
            .any(|entry| entry.device_id == device.id && entry.display_id == display.id)
        {
            let mut rect = Rect {
                x: anchor.0 + display.bounds.x,
                y: anchor.1 + display.bounds.y,
                width: display.bounds.width,
                height: display.bounds.height,
            };
            // Newly attached monitors must not overwrite user placements.
            if layout.iter().any(|d| !d.hidden && overlap(&d.rect, &rect)) {
                rect.x = layout
                    .iter()
                    .filter(|d| !d.hidden)
                    .map(|d| d.rect.x + d.rect.width)
                    .fold(0., f64::max);
            }
            layout.push(LayoutDisplay {
                device_id: device.id.clone(),
                display_id: display.id.clone(),
                rect,
                hidden: false,
            });
        }
    }
    if !layout.iter().any(|entry| !entry.hidden) {
        if let Some(entry) = layout.iter_mut().find(|entry| entry.device_id == device.id) {
            entry.hidden = false;
        }
    }
}
fn overlap(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.width - 0.5
        && a.x + a.width > b.x + 0.5
        && a.y < b.y + b.height - 0.5
        && a.y + a.height > b.y + 0.5
}
/// An offset/stacked display can only be crossed along its actual shared edge.
pub fn adjacent<'a>(
    layout: &'a [LayoutDisplay],
    from: &LayoutDisplay,
    x: f64,
    y: f64,
    dx: f64,
    dy: f64,
) -> Option<&'a LayoutDisplay> {
    if ![x, y, dx, dy].iter().all(|v| v.is_finite()) {
        return None;
    }
    layout
        .iter()
        .filter(|d| {
            !d.hidden && !(d.device_id == from.device_id && d.display_id == from.display_id)
        })
        .find(|d| {
            let a = &from.rect;
            let b = &d.rect;
            let eps = 2.0;
            (dx > 0.0
                && x >= a.x + a.width - 2.0
                && (b.x - (a.x + a.width)).abs() <= eps
                && y >= b.y
                && y < b.y + b.height)
                || (dx < 0.0
                    && x <= a.x + 1.0
                    && (a.x - (b.x + b.width)).abs() <= eps
                    && y >= b.y
                    && y < b.y + b.height)
                || (dy > 0.0
                    && y >= a.y + a.height - 2.0
                    && (b.y - (a.y + a.height)).abs() <= eps
                    && x >= b.x
                    && x < b.x + b.width)
                || (dy < 0.0
                    && y <= a.y + 1.0
                    && (a.y - (b.y + b.height)).abs() <= eps
                    && x >= b.x
                    && x < b.x + b.width)
        })
}
pub fn to_native(layout: &Rect, native: &Rect, x: f64, y: f64) -> (f64, f64) {
    native.clamp(
        native.x + (x - layout.x) / layout.width * native.width,
        native.y + (y - layout.y) / layout.height * native.height,
    )
}
pub fn to_workspace(layout: &Rect, native: &Rect, x: f64, y: f64) -> (f64, f64) {
    (
        layout.x + (x - native.x) / native.width * layout.width,
        layout.y + (y - native.y) / native.height * layout.height,
    )
}
pub fn map_key(code: u16, source: &str, target: &str, enabled: bool) -> u16 {
    if !enabled || source == target {
        return code;
    }
    if source == "macos" && target != "macos" {
        match code {
            227 => 224,
            231 => 228,
            224 => 227,
            228 => 231,
            _ => code,
        }
    } else if source != "macos" && target == "macos" {
        match code {
            224 => 227,
            228 => 231,
            227 => 224,
            231 => 228,
            _ => code,
        }
    } else {
        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_device_keeps_native_monitor_geometry_despite_enumeration_order() {
        let mut layout = vec![d("mac", 0., 0.)];
        let device = crate::model::Device {
            id: "pc".into(),
            name: "PC".into(),
            platform: "windows".into(),
            displays: vec![
                crate::model::Display {
                    id: "right".into(),
                    name: "Right".into(),
                    bounds: Rect {
                        x: 1920.,
                        y: 200.,
                        width: 1920.,
                        height: 1080.,
                    },
                    scale: 1.,
                    primary: false,
                },
                crate::model::Display {
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
            ],
        };
        reconcile_device(&mut layout, &device);
        let primary = layout.iter().find(|d| d.display_id == "primary").unwrap();
        let right = layout.iter().find(|d| d.display_id == "right").unwrap();
        assert_eq!((primary.rect.x, primary.rect.y), (1920., 0.));
        assert_eq!((right.rect.x, right.rect.y), (3840., 200.));
        assert_eq!(
            adjacent(&layout, &layout[0], 1919., 500., 2., 0.)
                .unwrap()
                .display_id,
            "primary"
        );
    }
    fn d(id: &str, x: f64, y: f64) -> LayoutDisplay {
        LayoutDisplay {
            device_id: id.into(),
            display_id: "main".into(),
            rect: Rect {
                x,
                y,
                width: 1920.,
                height: 1080.,
            },
            hidden: false,
        }
    }
    #[test]
    fn offset_crossing() {
        let a = d("a", 0., 0.);
        let b = d("b", 1920., 500.);
        let l = vec![a.clone(), b];
        assert!(adjacent(&l, &a, 1919., 200., 1., 0.).is_none());
        assert_eq!(
            adjacent(&l, &a, 1919., 600., 1., 0.).unwrap().device_id,
            "b"
        );
    }
    #[test]
    fn hidden_and_gap() {
        let a = d("a", 0., 0.);
        let mut b = d("b", 1920., 0.);
        b.hidden = true;
        assert!(adjacent(&[a.clone(), b], &a, 1919., 600., 1., 0.).is_none());
        assert!(adjacent(&[a.clone(), d("b", 1950., 0.)], &a, 1919., 600., 1., 0.).is_none());
    }
    #[test]
    fn rejects_overlap_nan_and_all_hidden() {
        assert!(validate(&[d("a", 0., 0.), d("b", 100., 0.)]).is_err());
        let mut a = d("a", 0., 0.);
        a.rect.x = f64::NAN;
        assert!(validate(&[a.clone()]).is_err());
        a.rect.x = 0.;
        a.hidden = true;
        assert!(validate(&[a]).is_err());
    }
    #[test]
    fn scaling_and_modifiers() {
        let a = Rect {
            x: 0.,
            y: 0.,
            width: 1920.,
            height: 1080.,
        };
        let b = Rect {
            x: -2560.,
            y: 0.,
            width: 2560.,
            height: 1440.,
        };
        assert_eq!(to_native(&a, &b, 960., 540.), (-1280., 720.));
        assert_eq!(map_key(227, "macos", "windows", true), 224);
        assert_eq!(map_key(224, "windows", "macos", true), 227);
    }
}
