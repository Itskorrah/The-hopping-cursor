//! Scroll units on the wire are fractional wheel/line steps. Positive Y is
//! wheel-forward (traditional up); positive X is Windows wheel-right.
//! Quartz exposes application-direction deltas, so Mac's preference is applied
//! at the receiving boundary and undone at the source boundary.

#[derive(Default)]
#[cfg(any(target_os = "macos", test))]
pub(super) struct MacScroll {
    lines: [f64; 2],
    points: [f64; 2],
    natural: Option<bool>,
}

#[cfg(any(target_os = "macos", test))]
pub(super) struct MacDeltas {
    pub lines: [i32; 2],
    pub fixed: [f64; 2],
    pub points: [i64; 2],
}

#[cfg(any(target_os = "macos", test))]
impl MacScroll {
    pub fn prepare(&mut self, x: f64, y: f64, natural: bool, points_per_line: f64) -> MacDeltas {
        if self.natural != Some(natural) {
            self.reset();
            self.natural = Some(natural);
        }
        let fixed = mac_direction(x, y, natural);
        let mut delta = MacDeltas {
            lines: [0; 2],
            fixed,
            points: [0; 2],
        };
        for (axis, value) in fixed.into_iter().enumerate() {
            self.lines[axis] += value;
            delta.lines[axis] = whole(self.lines[axis]) as i32;
            self.lines[axis] -= f64::from(delta.lines[axis]);
            self.points[axis] += value * points_per_line;
            delta.points[axis] = whole(self.points[axis]) as i64;
            self.points[axis] -= delta.points[axis] as f64;
        }
        delta
    }
    pub fn reset(&mut self) {
        self.lines = [0.; 2];
        self.points = [0.; 2];
    }
}

fn whole(value: f64) -> f64 {
    // Accumulating 120 fractions of 1/120 must yield one whole step, rather
    // than 0.9999999999999989. Only absorb floating-point roundoff at a boundary.
    let nearest = value.round();
    if (value - nearest).abs() < 1e-9 {
        nearest
    } else {
        value.trunc()
    }
}

#[cfg(any(target_os = "macos", test))]
pub(super) fn mac_direction(x: f64, y: f64, natural: bool) -> [f64; 2] {
    if natural {
        [x, -y]
    } else {
        [-x, y]
    }
}

#[cfg(any(target_os = "windows", test))]
#[derive(Default)]
pub(super) struct WindowsScroll {
    units: [f64; 2],
}
#[cfg(any(target_os = "windows", test))]
impl WindowsScroll {
    pub fn prepare(&mut self, x: f64, y: f64) -> [i32; 2] {
        let mut output = [0; 2];
        for (axis, value) in [x, y].into_iter().enumerate() {
            self.units[axis] += value * 120.;
            output[axis] = whole(self.units[axis]) as i32;
            self.units[axis] -= f64::from(output[axis]);
        }
        output
    }
    pub fn reset(&mut self) {
        self.units = [0.; 2];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_preserves_sub_unit_wheels_without_changing_axis_direction() {
        let mut scroll = WindowsScroll::default();
        let mut total = [0; 2];
        for _ in 0..512 {
            let delta = scroll.prepare(1. / (120. * 512.), -1. / (120. * 512.));
            total[0] += delta[0];
            total[1] += delta[1];
        }
        assert_eq!(total, [1, -1]);
        assert_eq!(scroll.prepare(0.125, -0.125), [15, -15]);
        scroll.prepare(0., 0.5 / 120.);
        assert_eq!(scroll.prepare(0., -0.5 / 120.), [0, 0]);
        scroll.prepare(0., 0.5 / 120.);
        scroll.reset();
        assert_eq!(scroll.prepare(0., 0.5 / 120.), [0, 0]);
    }

    #[test]
    fn fractional_wheels_accumulate_without_discarding_small_movements() {
        let mut scroll = MacScroll::default();
        let mut lines = [0; 2];
        let mut points = [0; 2];
        for _ in 0..8 {
            let delta = scroll.prepare(0.125, 0.125, false, 8.);
            assert_eq!(delta.fixed, [-0.125, 0.125]);
            for axis in 0..2 {
                lines[axis] += delta.lines[axis];
                points[axis] += delta.points[axis];
            }
        }
        assert_eq!(lines, [-1, 1]);
        assert_eq!(points, [-8, 8]);
    }

    #[test]
    fn destination_natural_preference_reverses_both_axes() {
        let mut scroll = MacScroll::default();
        let standard = scroll.prepare(1., -2., false, 8.);
        assert_eq!(standard.lines, [-1, -2]);
        assert_eq!(standard.points, [-8, -16]);
        let natural = scroll.prepare(1., -2., true, 8.);
        assert_eq!(natural.lines, [1, 2]);
        assert_eq!(natural.points, [8, 16]);
    }

    #[test]
    fn release_and_direction_changes_do_not_leak_fractional_state() {
        let mut scroll = MacScroll::default();
        scroll.prepare(0., 0.5, false, 8.);
        scroll.reset();
        assert_eq!(scroll.prepare(0., 0.5, false, 8.).lines, [0, 0]);
        assert_eq!(scroll.prepare(0., -0.5, false, 8.).lines, [0, 0]);
        assert_eq!(scroll.prepare(0., -1., false, 8.).lines, [0, -1]);
        scroll.prepare(0., 0.5, false, 8.);
        assert_eq!(scroll.prepare(0., 0.5, true, 8.).lines, [0, 0]);
    }

    #[test]
    fn source_normalization_and_destination_policy_do_not_double_invert() {
        assert_eq!(mac_direction(-0.5, 1., false), [0.5, 1.]);
        assert_eq!(mac_direction(0.5, -1., true), [0.5, 1.]);
        assert_eq!(
            MacScroll::default().prepare(0.5, 1., true, 8.).fixed,
            [0.5, -1.]
        );
        assert_eq!(
            MacScroll::default().prepare(0.5, 1., false, 8.).fixed,
            [-0.5, 1.]
        );
    }

    #[test]
    fn sub_notch_packets_do_not_lose_motion_to_floating_point_roundoff() {
        let mut scroll = MacScroll::default();
        let mut total = [0_i64; 2];
        for _ in 0..120 {
            let delta = scroll.prepare(0., 1. / 120., false, 8.);
            total[0] += i64::from(delta.lines[1]);
            total[1] += delta.points[1];
        }
        assert_eq!(total, [1, 8]);
    }
}
