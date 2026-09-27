//! Pixels for the tray icon: the Claude mark inside a thin ring that fills with the share of budget
//! used (blue, yellow from 75%, red from 90%). As a template image (macOS) it's black with alpha for
//! the system to tint; otherwise it uses the mark's and the theme's colors.

pub const SIZE: u32 = 36;
const SUPERSAMPLE: u32 = 4;
const INNER: f64 = 0.395;
const OUTER: f64 = 0.485;

/// The Claude mark, raw RGBA, `MARK_SIZE` × `MARK_SIZE` (built by tools/icons/make_icons.py).
const MARK: &[u8] = include_bytes!("claude_mark.rgba");
const MARK_SIZE: u32 = 20;

/// RGBA pixels, `SIZE` × `SIZE`.
pub fn ring_rgba(fraction: Option<f64>, template: bool) -> Vec<u8> {
    let filled = fraction.unwrap_or(0.0).clamp(0.0, 1.0);
    let (track, fill) = if template {
        ([0, 0, 0, 90], [0, 0, 0, 255])
    } else {
        ([128, 128, 128, 150], severity(fraction.unwrap_or(0.0)))
    };

    let n = SIZE as f64;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let mut acc = [0u32; 4];
            for sy in 0..SUPERSAMPLE {
                for sx in 0..SUPERSAMPLE {
                    let fx = (x as f64 + (sx as f64 + 0.5) / SUPERSAMPLE as f64) / n - 0.5;
                    let fy = (y as f64 + (sy as f64 + 0.5) / SUPERSAMPLE as f64) / n - 0.5;
                    let d = (fx * fx + fy * fy).sqrt();
                    if !(INNER..=OUTER).contains(&d) {
                        continue;
                    }
                    // Angle clockwise from 12 o'clock, as a fraction of a full turn.
                    let turn = (fx.atan2(-fy).to_degrees() + 360.0) % 360.0 / 360.0;
                    let c = if turn <= filled && filled > 0.0 { fill } else { track };
                    for i in 0..4 {
                        acc[i] += c[i] as u32;
                    }
                }
            }
            let samples = SUPERSAMPLE * SUPERSAMPLE;
            let i = ((y * SIZE + x) * 4) as usize;
            for c in 0..4 {
                rgba[i + c] = (acc[c] / samples) as u8;
            }
        }
    }
    draw_mark(&mut rgba, template);
    rgba
}

/// The mark, centered inside the ring (they don't overlap).
fn draw_mark(rgba: &mut [u8], template: bool) {
    let offset = (SIZE - MARK_SIZE) / 2;
    for y in 0..MARK_SIZE {
        for x in 0..MARK_SIZE {
            let s = ((y * MARK_SIZE + x) * 4) as usize;
            let d = (((y + offset) * SIZE + x + offset) * 4) as usize;
            let alpha = MARK[s + 3];
            if template {
                rgba[d..d + 4].copy_from_slice(&[0, 0, 0, alpha]);
            } else {
                rgba[d..d + 4].copy_from_slice(&MARK[s..s + 4]);
            }
        }
    }
}

/// Theme meter colors (design/theme.css, dark values).
fn severity(fraction: f64) -> [u8; 4] {
    if fraction >= 0.90 {
        [255, 69, 58, 255]
    } else if fraction >= 0.75 {
        [255, 214, 10, 255]
    } else {
        [10, 132, 255, 255]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_asset_matches_its_size() {
        assert_eq!(MARK.len(), (MARK_SIZE * MARK_SIZE * 4) as usize);
    }

    #[test]
    fn ring_and_mark_are_drawn() {
        let px = ring_rgba(Some(0.5), false);
        assert_eq!(px.len(), (SIZE * SIZE * 4) as usize);
        let center = ((SIZE / 2 * SIZE + SIZE / 2) * 4) as usize;
        assert!(px[center + 3] > 0, "mark in the middle");
        let top = ((SIZE + SIZE / 2) * 4) as usize; // row 1, 12 o'clock: filled ring (blue)
        assert!(px[top + 2] > px[top], "filled ring is blue at the top");
    }
}
