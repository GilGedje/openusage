//! Pixels for the tray icon: a ring that fills with the share of budget used. As a template image
//! (macOS) it's black with alpha for the system to tint; otherwise it uses the theme's meter colors.

pub const SIZE: u32 = 36;
const SUPERSAMPLE: u32 = 4;
const INNER: f64 = 0.27;
const OUTER: f64 = 0.44;

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
    rgba
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
