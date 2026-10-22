use crate::protocol::Rgb;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsv {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

impl Hsv {
    pub fn from_rgb(rgb: Rgb) -> Self {
        let r = f32::from(rgb.r) / 255.0;
        let g = f32::from(rgb.g) / 255.0;
        let b = f32::from(rgb.b) / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let h = if delta <= f32::EPSILON {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };

        Self {
            h: if h < 0.0 { h + 360.0 } else { h },
            s: if max <= f32::EPSILON {
                0.0
            } else {
                delta / max
            },
            v: max,
        }
    }

    pub fn from_rgb_with_hue(rgb: Rgb, hue: f32) -> Self {
        let mut hsv = Self::from_rgb(rgb);

        if hsv.s <= f32::EPSILON || hsv.v <= f32::EPSILON {
            hsv.h = hue;
        }

        hsv
    }

    pub fn to_rgb(self) -> Rgb {
        let h = self.h.rem_euclid(360.0);
        let s = self.s.clamp(0.0, 1.0);
        let v = self.v.clamp(0.0, 1.0);

        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r, g, b) = match h {
            value if value < 60.0 => (c, x, 0.0),
            value if value < 120.0 => (x, c, 0.0),
            value if value < 180.0 => (0.0, c, x),
            value if value < 240.0 => (0.0, x, c),
            value if value < 300.0 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        Rgb::new(to_u8(r + m), to_u8(g + m), to_u8(b + m))
    }
}

fn to_u8(value: f32) -> u8 {
    (value * 255.0).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::Hsv;
    use crate::protocol::Rgb;

    #[test]
    fn converts_primary_colors() {
        assert_eq!(Hsv::from_rgb(Rgb::new(255, 0, 0)).h, 0.0);
        assert_eq!(Hsv::from_rgb(Rgb::new(0, 255, 0)).h, 120.0);
        assert_eq!(Hsv::from_rgb(Rgb::new(0, 0, 255)).h, 240.0);
        assert_eq!(Hsv::from_rgb(Rgb::WHITE).s, 0.0);
        assert_eq!(Hsv::from_rgb(Rgb::new(0, 0, 0)).v, 0.0);
    }

    #[test]
    fn creates_colors_from_hsv() {
        assert_eq!(
            Hsv {
                h: 0.0,
                s: 1.0,
                v: 1.0
            }
            .to_rgb(),
            Rgb::new(255, 0, 0)
        );
        assert_eq!(
            Hsv {
                h: 120.0,
                s: 1.0,
                v: 1.0
            }
            .to_rgb(),
            Rgb::new(0, 255, 0)
        );
        assert_eq!(
            Hsv {
                h: 240.0,
                s: 1.0,
                v: 1.0
            }
            .to_rgb(),
            Rgb::new(0, 0, 255)
        );
        assert_eq!(
            Hsv {
                h: 0.0,
                s: 0.0,
                v: 1.0
            }
            .to_rgb(),
            Rgb::WHITE
        );
    }

    #[test]
    fn round_trips_common_colors() {
        for color in [
            Rgb::new(0xFF, 0x5A, 0x36),
            Rgb::new(0x00, 0xFF, 0x88),
            Rgb::new(0x9D, 0x72, 0xFF),
            Rgb::new(0x3B, 0x82, 0xF6),
            Rgb::new(0x80, 0x80, 0x80),
        ] {
            assert_eq!(Hsv::from_rgb(color).to_rgb(), color);
        }
    }

    #[test]
    fn preserves_hue_for_achromatic_colors() {
        let hsv = Hsv::from_rgb_with_hue(Rgb::WHITE, 210.0);
        assert_eq!(hsv.h, 210.0);
        assert_eq!(hsv.to_rgb(), Rgb::WHITE);
    }
}
