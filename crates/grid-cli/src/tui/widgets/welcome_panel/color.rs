//! Color utilities: HSL to RGB conversion and pseudo-random generator.

use ratatui::style::Color;

/// Convert HSL to ratatui `Color::Rgb`. Hue in 0..360, saturation/lightness in 0.0..1.0.
pub(super) fn hsl_to_rgb(hue: f64, saturation: f64, lightness: f64) -> Color {
    let c = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let h = hue / 60.0;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = lightness - c / 2.0;
    Color::Rgb(
        ((r1 + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).clamp(0.0, 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hsl_primary_colors() {
        let Color::Rgb(r, g, b) = hsl_to_rgb(0.0, 1.0, 0.5) else {
            panic!("expected Rgb");
        };
        assert_eq!(r, 255);
        assert!(g < 5);
        assert!(b < 5);
    }

    #[test]
    fn test_hsl_amber() {
        let Color::Rgb(r, g, _b) = hsl_to_rgb(40.0, 0.8, 0.5) else {
            panic!("expected Rgb");
        };
        // Amber: red > green > blue
        assert!(r > 200);
        assert!(g > 100);
    }
}
