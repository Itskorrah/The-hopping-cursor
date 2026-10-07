/// Windows virtual key to USB HID usage. Left/right modifiers remain distinct.
#[allow(dead_code)]
pub fn windows_to_hid(vk: u32, scan: u32, extended: bool) -> Option<u16> {
    if scan != 0 {
        if let Some(hid) = SCAN_KEYS
            .iter()
            .find(|(s, e, _)| *s == scan as u16 && *e == extended)
            .map(|(_, _, h)| *h)
        {
            return Some(hid);
        }
    }
    match vk {
        0x41..=0x5a => Some((vk - 0x41 + 4) as u16),
        0x31..=0x39 => Some((vk - 0x31 + 30) as u16),
        0x30 => Some(39),
        0x70..=0x7b => Some((vk - 0x70 + 58) as u16),
        0x0d => Some(if extended { 88 } else { 40 }),
        0x1b => Some(41),
        0x08 => Some(42),
        0x09 => Some(43),
        0x20 => Some(44),
        0xbd => Some(45),
        0xbb => Some(46),
        0xdb => Some(47),
        0xdd => Some(48),
        0xdc => Some(49),
        0xba => Some(51),
        0xde => Some(52),
        0xc0 => Some(53),
        0xbc => Some(54),
        0xbe => Some(55),
        0xbf => Some(56),
        0x14 => Some(57),
        0x2c => Some(70),
        0x91 => Some(71),
        0x13 => Some(72),
        0x2d => Some(73),
        0x24 => Some(74),
        0x21 => Some(75),
        0x2e => Some(76),
        0x23 => Some(77),
        0x22 => Some(78),
        0x27 => Some(79),
        0x25 => Some(80),
        0x28 => Some(81),
        0x26 => Some(82),
        0x90 => Some(83),
        0x6f => Some(84),
        0x6a => Some(85),
        0x6d => Some(86),
        0x6b => Some(87),
        0x61..=0x69 => Some((vk - 0x61 + 89) as u16),
        0x60 => Some(98),
        0x6e => Some(99),
        0x5d => Some(101),
        0xa0 => Some(225),
        0xa1 => Some(229),
        0xa2 => Some(224),
        0xa3 => Some(228),
        0xa4 => Some(226),
        0xa5 => Some(230),
        0x5b => Some(227),
        0x5c => Some(231),
        0x10 => Some(if scan == 0x36 { 229 } else { 225 }),
        0x11 => Some(if extended { 228 } else { 224 }),
        0x12 => Some(if extended { 230 } else { 226 }),
        _ => None,
    }
}
/// PC set-1 scan codes preserve physical key positions across keyboard layouts.
#[allow(dead_code)]
pub const SCAN_KEYS: &[(u16, bool, u16)] = &[
    (1, false, 41),
    (2, false, 30),
    (3, false, 31),
    (4, false, 32),
    (5, false, 33),
    (6, false, 34),
    (7, false, 35),
    (8, false, 36),
    (9, false, 37),
    (10, false, 38),
    (11, false, 39),
    (12, false, 45),
    (13, false, 46),
    (14, false, 42),
    (15, false, 43),
    (16, false, 20),
    (17, false, 26),
    (18, false, 8),
    (19, false, 21),
    (20, false, 23),
    (21, false, 28),
    (22, false, 24),
    (23, false, 12),
    (24, false, 18),
    (25, false, 19),
    (26, false, 47),
    (27, false, 48),
    (28, false, 40),
    (29, false, 224),
    (30, false, 4),
    (31, false, 22),
    (32, false, 7),
    (33, false, 9),
    (34, false, 10),
    (35, false, 11),
    (36, false, 13),
    (37, false, 14),
    (38, false, 15),
    (39, false, 51),
    (40, false, 52),
    (41, false, 53),
    (42, false, 225),
    (43, false, 49),
    (44, false, 29),
    (45, false, 27),
    (46, false, 6),
    (47, false, 25),
    (48, false, 5),
    (49, false, 17),
    (50, false, 16),
    (51, false, 54),
    (52, false, 55),
    (53, false, 56),
    (54, false, 229),
    (55, false, 85),
    (56, false, 226),
    (57, false, 44),
    (58, false, 57),
    (59, false, 58),
    (60, false, 59),
    (61, false, 60),
    (62, false, 61),
    (63, false, 62),
    (64, false, 63),
    (65, false, 64),
    (66, false, 65),
    (67, false, 66),
    (68, false, 67),
    (69, false, 83),
    (70, false, 71),
    (71, false, 95),
    (72, false, 96),
    (73, false, 97),
    (74, false, 86),
    (75, false, 92),
    (76, false, 93),
    (77, false, 94),
    (78, false, 87),
    (79, false, 89),
    (80, false, 90),
    (81, false, 91),
    (82, false, 98),
    (83, false, 99),
    (86, false, 100),
    (87, false, 68),
    (88, false, 69),
    (28, true, 88),
    (29, true, 228),
    (53, true, 84),
    (55, true, 70),
    (56, true, 230),
    (71, true, 74),
    (72, true, 82),
    (73, true, 75),
    (75, true, 80),
    (77, true, 79),
    (79, true, 77),
    (80, true, 81),
    (81, true, 78),
    (82, true, 73),
    (83, true, 76),
    (91, true, 227),
    (92, true, 231),
    (93, true, 101),
];
#[allow(dead_code)]
pub fn hid_to_windows_scan(h: u16) -> Option<(u16, bool)> {
    SCAN_KEYS
        .iter()
        .find(|(_, _, v)| *v == h)
        .map(|(s, e, _)| (*s, *e))
}
#[allow(dead_code)]
pub fn hid_to_windows(h: u16) -> Option<(u16, bool)> {
    for vk in 0..=255 {
        if windows_to_hid(vk, 0, false) == Some(h) {
            return Some((
                vk as u16,
                matches!(h, 70 | 73..=84 | 101 | 227 | 231 | 228 | 230),
            ));
        }
    }
    if h == 88 {
        Some((13, true))
    } else {
        None
    }
}
// Apple virtual keycodes are physical positions; the receiving layout supplies glyphs.
#[allow(dead_code)]
pub const MAC_KEYS: &[(u16, u16)] = &[
    (0, 4),
    (1, 22),
    (2, 7),
    (3, 9),
    (4, 11),
    (5, 10),
    (6, 29),
    (7, 27),
    (8, 6),
    (9, 25),
    (11, 5),
    (12, 20),
    (13, 26),
    (14, 8),
    (15, 21),
    (16, 28),
    (17, 23),
    (18, 30),
    (19, 31),
    (20, 32),
    (21, 33),
    (22, 35),
    (23, 34),
    (24, 46),
    (25, 38),
    (26, 36),
    (27, 45),
    (28, 37),
    (29, 39),
    (30, 48),
    (31, 18),
    (32, 24),
    (33, 47),
    (34, 12),
    (35, 19),
    (36, 40),
    (37, 15),
    (38, 13),
    (39, 52),
    (40, 14),
    (41, 51),
    (42, 49),
    (43, 54),
    (44, 56),
    (45, 17),
    (46, 16),
    (47, 55),
    (48, 43),
    (49, 44),
    (50, 53),
    (51, 42),
    (53, 41),
    (54, 231),
    (55, 227),
    (56, 225),
    (57, 57),
    (58, 226),
    (59, 224),
    (60, 229),
    (61, 230),
    (62, 228),
    (65, 99),
    (67, 85),
    (69, 87),
    (75, 84),
    (76, 88),
    (78, 86),
    (81, 103),
    (82, 98),
    (83, 89),
    (84, 90),
    (85, 91),
    (86, 92),
    (87, 93),
    (88, 94),
    (89, 95),
    (91, 96),
    (92, 97),
    (96, 62),
    (97, 63),
    (98, 64),
    (99, 60),
    (100, 65),
    (101, 66),
    (103, 68),
    (105, 104),
    (106, 107),
    (107, 105),
    (109, 67),
    (111, 69),
    (113, 106),
    (114, 73),
    (115, 74),
    (116, 75),
    (117, 76),
    (118, 61),
    (119, 77),
    (120, 59),
    (121, 78),
    (122, 58),
    (123, 80),
    (124, 79),
    (125, 81),
    (126, 82),
];
#[allow(dead_code)]
pub fn mac_to_hid(code: u16) -> Option<u16> {
    MAC_KEYS.iter().find(|(m, _)| *m == code).map(|(_, h)| *h)
}
#[allow(dead_code)]
pub fn hid_to_mac(code: u16) -> Option<u16> {
    MAC_KEYS.iter().find(|(_, h)| *h == code).map(|(m, _)| *m)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_scan_wins_over_source_layout() {
        assert_eq!(windows_to_hid(0x41, 16, false), Some(20));
        for (s, e, h) in SCAN_KEYS {
            assert_eq!(windows_to_hid(0, *s as u32, *e), Some(*h));
            assert_eq!(hid_to_windows_scan(*h), Some((*s, *e)));
        }
    }
    #[test]
    fn key_roundtrips() {
        for h in [
            4, 29, 30, 39, 40, 41, 44, 58, 69, 79, 88, 224, 225, 226, 227, 228, 229, 230, 231,
        ] {
            let (v, e) = hid_to_windows(h).unwrap();
            assert_eq!(windows_to_hid(v as u32, 0, e), Some(h), "HID {h}");
            let m = hid_to_mac(h).unwrap();
            assert_eq!(mac_to_hid(m), Some(h));
        }
    }
}
