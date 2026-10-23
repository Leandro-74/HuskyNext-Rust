use std::fmt;

pub const REPORT_ID: u8 = 0x04;
pub const REPORT_LEN: usize = 64;
pub const MODE_OFFSET: usize = 9;
pub const BRIGHT_OFFSET: usize = 10;
pub const SPEED_OFFSET: usize = 11;
pub const COLOR_OFFSET_R: usize = 14;
pub const COLOR_OFFSET_G: usize = 15;
pub const COLOR_OFFSET_B: usize = 16;

pub const MAX_BRIGHTNESS: u8 = 4;
pub const MIN_SPEED: u8 = 1;
pub const MAX_SPEED: u8 = 5;
pub const DEFAULT_SPEED: u8 = 4;

pub const BASE_REPORT: [u8; REPORT_LEN] = [
    0x04, 0xA1, 0x01, 0x06, 0x38, 0x00, 0x00, 0x55, 0x00, 0x06, 0x04, 0x01, 0x00, 0x00, 0xFF, 0xFF,
    0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Effect {
    Off,
    Wave,
    Blinds,
    Diffusion,
    Neon,
    Breathing,
    #[default]
    Static,
    Reactive,
    Raindrop,
    LineReactive,
    Starry,
    Blooming,
    Cascade,
    Rainbow,
    Cartoon,
    RainScene,
    Scan,
    SingleOff,
    Flow,
}

impl Effect {
    pub const ALL: [Self; 19] = [
        Self::Off,
        Self::Wave,
        Self::Blinds,
        Self::Diffusion,
        Self::Neon,
        Self::Breathing,
        Self::Static,
        Self::Reactive,
        Self::Raindrop,
        Self::LineReactive,
        Self::Starry,
        Self::Blooming,
        Self::Cascade,
        Self::Rainbow,
        Self::Cartoon,
        Self::RainScene,
        Self::Scan,
        Self::SingleOff,
        Self::Flow,
    ];

    pub const fn code(self) -> u8 {
        match self {
            Self::Off => 0x00,
            Self::Wave => 0x01,
            Self::Blinds => 0x02,
            Self::Diffusion => 0x03,
            Self::Neon => 0x04,
            Self::Breathing => 0x05,
            Self::Static => 0x06,
            Self::Reactive => 0x07,
            Self::Raindrop => 0x08,
            Self::LineReactive => 0x09,
            Self::Starry => 0x0A,
            Self::Blooming => 0x0B,
            Self::Cascade => 0x0C,
            Self::Rainbow => 0x0D,
            Self::Cartoon => 0x0E,
            Self::RainScene => 0x0F,
            Self::Scan => 0x10,
            Self::SingleOff => 0x11,
            Self::Flow => 0x12,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Off => "Desligado",
            Self::Wave => "Onda",
            Self::Blinds => "Persianas",
            Self::Diffusion => "Difusão",
            Self::Neon => "Néon",
            Self::Breathing => "Respiração",
            Self::Static => "Estático",
            Self::Reactive => "Reativo",
            Self::Raindrop => "Gota de Chuva",
            Self::LineReactive => "Reativa por Linha",
            Self::Starry => "Iluminado por Estrelas",
            Self::Blooming => "Flores em Flor",
            Self::Cascade => "Cascata",
            Self::Rainbow => "Arco-íris",
            Self::Cartoon => "Desenho Animado",
            Self::RainScene => "Cena de Chuva",
            Self::Scan => "Escaneamento",
            Self::SingleOff => "Single Off",
            Self::Flow => "Fluxo",
        }
    }

    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0x00 => Some(Self::Off),
            0x01 => Some(Self::Wave),
            0x02 => Some(Self::Blinds),
            0x03 => Some(Self::Diffusion),
            0x04 => Some(Self::Neon),
            0x05 => Some(Self::Breathing),
            0x06 => Some(Self::Static),
            0x07 => Some(Self::Reactive),
            0x08 => Some(Self::Raindrop),
            0x09 => Some(Self::LineReactive),
            0x0A => Some(Self::Starry),
            0x0B => Some(Self::Blooming),
            0x0C => Some(Self::Cascade),
            0x0D => Some(Self::Rainbow),
            0x0E => Some(Self::Cartoon),
            0x0F => Some(Self::RainScene),
            0x10 => Some(Self::Scan),
            0x11 => Some(Self::SingleOff),
            0x12 => Some(Self::Flow),
            _ => None,
        }
    }
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const WHITE: Self = Self {
        r: 0xFF,
        g: 0xFF,
        b: 0xFF,
    };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(input: &str) -> Result<Self, ColorError> {
        let value = input.trim();
        let value = value.strip_prefix('#').unwrap_or(value);

        if value.len() != 6 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(ColorError::InvalidFormat);
        }

        let r = u8::from_str_radix(&value[0..2], 16).map_err(|_| ColorError::InvalidFormat)?;
        let g = u8::from_str_radix(&value[2..4], 16).map_err(|_| ColorError::InvalidFormat)?;
        let b = u8::from_str_radix(&value[4..6], 16).map_err(|_| ColorError::InvalidFormat)?;

        Ok(Self { r, g, b })
    }

    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

impl Default for Rgb {
    fn default() -> Self {
        Self::WHITE
    }
}

pub const CONNECTION_TEST_COLOR: Rgb = Rgb::new(0x88, 0x35, 0xE9);
pub const CONNECTION_TEST_HEX: &str = "8835E9";

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ColorError {
    #[error("Cor inválida. Use seis dígitos hexadecimais, como FF5733 ou #FF5733.")]
    InvalidFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardState {
    pub effect: Effect,
    pub brightness: u8,
    pub speed: u8,
    pub color: Rgb,
}

impl Default for KeyboardState {
    fn default() -> Self {
        Self {
            effect: Effect::Static,
            brightness: MAX_BRIGHTNESS,
            speed: DEFAULT_SPEED,
            color: Rgb::WHITE,
        }
    }
}

impl KeyboardState {
    pub fn sanitized(effect: u8, brightness: u8, speed: u8, r: u8, g: u8, b: u8) -> Self {
        Self {
            effect: Effect::from_code(effect).unwrap_or_default(),
            brightness: brightness.min(MAX_BRIGHTNESS),
            speed: speed.clamp(MIN_SPEED, MAX_SPEED),
            color: Rgb::new(r, g, b),
        }
    }

    pub fn report(self) -> [u8; REPORT_LEN] {
        let mut report = BASE_REPORT;
        report[MODE_OFFSET] = self.effect.code();
        report[BRIGHT_OFFSET] = self.brightness.min(MAX_BRIGHTNESS);
        report[SPEED_OFFSET] = speed_byte(self.speed);
        report[COLOR_OFFSET_R] = self.color.r;
        report[COLOR_OFFSET_G] = self.color.g;
        report[COLOR_OFFSET_B] = self.color.b;
        report
    }
}

pub const fn speed_byte(level: u8) -> u8 {
    let level = if level < MIN_SPEED {
        MIN_SPEED
    } else if level > MAX_SPEED {
        MAX_SPEED
    } else {
        level
    };
    MAX_SPEED - level
}

pub fn brightness_percent(level: u8) -> u8 {
    level.min(MAX_BRIGHTNESS) * 25
}

pub fn brightness_label(level: u8) -> String {
    format!("{}%", brightness_percent(level))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_report_is_well_formed() {
        assert_eq!(BASE_REPORT.len(), REPORT_LEN);
        assert_eq!(BASE_REPORT[0], REPORT_ID);
        assert_eq!(BASE_REPORT[MODE_OFFSET], Effect::Static.code());
        assert_eq!(BASE_REPORT[BRIGHT_OFFSET], MAX_BRIGHTNESS);
        assert_eq!(BASE_REPORT[SPEED_OFFSET], speed_byte(DEFAULT_SPEED));
        assert_eq!(
            &BASE_REPORT[COLOR_OFFSET_R..=COLOR_OFFSET_B],
            &[0xFF, 0xFF, 0xFF]
        );
    }

    #[test]
    fn speed_levels_map_to_reverse_bytes() {
        assert_eq!(speed_byte(MIN_SPEED), 0x04);
        assert_eq!(speed_byte(2), 0x03);
        assert_eq!(speed_byte(3), 0x02);
        assert_eq!(speed_byte(4), 0x01);
        assert_eq!(speed_byte(MAX_SPEED), 0x00);
        assert_eq!(speed_byte(0), speed_byte(MIN_SPEED));
        assert_eq!(speed_byte(200), speed_byte(MAX_SPEED));
    }

    #[test]
    fn default_state_matches_factory_report() {
        assert_eq!(KeyboardState::default().report(), BASE_REPORT);
    }

    #[test]
    fn custom_state_uses_expected_offsets() {
        let state = KeyboardState {
            effect: Effect::Rainbow,
            brightness: 2,
            speed: 5,
            color: Rgb::new(0xFF, 0x57, 0x33),
        };
        let report = state.report();

        assert_eq!(report[MODE_OFFSET], 0x0D);
        assert_eq!(report[BRIGHT_OFFSET], 0x02);
        assert_eq!(report[SPEED_OFFSET], 0x00);
        assert_eq!(report[COLOR_OFFSET_R], 0xFF);
        assert_eq!(report[COLOR_OFFSET_G], 0x57);
        assert_eq!(report[COLOR_OFFSET_B], 0x33);
        assert_eq!(
            &report[1..9],
            &[0xA1, 0x01, 0x06, 0x38, 0x00, 0x00, 0x55, 0x00]
        );
    }

    #[test]
    fn hex_round_trip() {
        for input in ["FF5733", "#ff5733", " 00AA10 "] {
            let color = Rgb::from_hex(input).unwrap();
            assert_eq!(Rgb::from_hex(&color.to_hex()).unwrap(), color);
        }
    }

    #[test]
    fn invalid_hex_is_rejected() {
        for input in ["", "#", "FFF", "GG0000", "FF 000", "##FF0000", "FF00000"] {
            assert!(Rgb::from_hex(input).is_err(), "{input:?} should fail");
        }
    }

    #[test]
    fn sanitization_clamps_invalid_values() {
        let state = KeyboardState::sanitized(0xFE, 99, 0, 1, 2, 3);
        assert_eq!(state.effect, Effect::Static);
        assert_eq!(state.brightness, MAX_BRIGHTNESS);
        assert_eq!(state.speed, MIN_SPEED);
        assert_eq!(state.color, Rgb::new(1, 2, 3));
    }

    #[test]
    fn every_effect_round_trips() {
        assert_eq!(Effect::ALL.len(), 19);
        for effect in Effect::ALL {
            assert_eq!(Effect::from_code(effect.code()), Some(effect));
        }
    }
}
