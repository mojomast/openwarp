//! Terminal color helpers.

/// xterm default foreground: a light gray that reads well on dark panes.
pub const DEFAULT_FOREGROUND: (u8, u8, u8) = (229, 231, 235);
/// xterm default background: near-black, matching the PTY panel shell.
pub const DEFAULT_BACKGROUND: (u8, u8, u8) = (8, 12, 18);

const ANSI_16: [(u8, u8, u8); 16] = [
    (0, 0, 0),
    (205, 49, 49),
    (13, 188, 121),
    (229, 229, 16),
    (36, 114, 200),
    (188, 63, 188),
    (17, 168, 205),
    (229, 229, 229),
    (102, 102, 102),
    (241, 76, 76),
    (35, 209, 139),
    (245, 245, 67),
    (59, 142, 234),
    (214, 112, 214),
    (41, 184, 219),
    (255, 255, 255),
];

/// Convert an xterm 256-color palette index to RGB.
pub fn xterm_256_color(index: u8) -> (u8, u8, u8) {
    match index {
        0..=15 => ANSI_16[index as usize],
        16..=231 => {
            let i = index - 16;
            let component = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            (component(i / 36), component((i % 36) / 6), component(i % 6))
        }
        232..=255 => {
            let v = 8 + (index - 232) * 10;
            (v, v, v)
        }
    }
}
