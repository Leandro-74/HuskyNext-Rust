use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use directories::BaseDirs;
use serde::{Deserialize, Serialize};

use crate::protocol::{Effect, KeyboardState, MAX_BRIGHTNESS, Rgb};

const CONFIG_DIR_NAME: &str = ".huskynext";
const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceConfig {
    pub vendor_id: u16,
    pub product_id: u16,
    pub interface_number: Option<i32>,
    pub usage_page: Option<u16>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub interface_number: Option<i32>,
    pub usage_page: Option<u16>,
    pub effect: Option<u8>,
    pub bright: Option<u8>,
    pub r: Option<u8>,
    pub g: Option<u8>,
    pub b: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct LoadResult {
    pub config: Config,
    pub warning: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Não foi possível localizar o diretório inicial do usuário.")]
    NoHome,
    #[error("Falha ao acessar {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Falha ao serializar a configuração: {0}")]
    Serialize(#[from] serde_json::Error),
}

impl Config {
    pub fn device(&self) -> Option<DeviceConfig> {
        let vendor_id = self.vendor_id?;
        let product_id = self.product_id?;

        if vendor_id == 0 || product_id == 0 {
            return None;
        }

        Some(DeviceConfig {
            vendor_id,
            product_id,
            interface_number: self.interface_number.filter(|iface| *iface >= 0),
            usage_page: self.usage_page,
        })
    }

    pub fn set_device(&mut self, device: DeviceConfig) {
        self.vendor_id = Some(device.vendor_id);
        self.product_id = Some(device.product_id);
        self.interface_number = device.interface_number;
        self.usage_page = device.usage_page;
    }

    pub fn clear_device(&mut self) {
        self.vendor_id = None;
        self.product_id = None;
        self.interface_number = None;
        self.usage_page = None;
    }

    pub fn keyboard_state(&self) -> KeyboardState {
        let defaults = KeyboardState::default();

        KeyboardState {
            effect: self
                .effect
                .and_then(Effect::from_code)
                .unwrap_or(defaults.effect),
            brightness: self.bright.unwrap_or(MAX_BRIGHTNESS).min(MAX_BRIGHTNESS),
            color: Rgb::new(
                self.r.unwrap_or(defaults.color.r),
                self.g.unwrap_or(defaults.color.g),
                self.b.unwrap_or(defaults.color.b),
            ),
        }
    }

    pub fn set_state(&mut self, state: KeyboardState) {
        self.effect = Some(state.effect.code());
        self.bright = Some(state.brightness.min(MAX_BRIGHTNESS));
        self.r = Some(state.color.r);
        self.g = Some(state.color.g);
        self.b = Some(state.color.b);
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let path = config_file().ok_or(ConfigError::NoHome)?;
        self.save_to(&path)
    }

    pub fn save_to(&self, path: &Path) -> Result<(), ConfigError> {
        let io = |source| ConfigError::Io {
            path: path.to_path_buf(),
            source,
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io)?;
        }

        let json = serde_json::to_vec_pretty(self)?;
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let temp_path = path.with_file_name(format!(
            ".{}.{}.{}.tmp",
            CONFIG_FILE_NAME,
            std::process::id(),
            unique
        ));

        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }

        let result = (|| {
            let mut file = options.open(&temp_path).map_err(io)?;
            file.write_all(&json).map_err(io)?;
            file.write_all(b"\n").map_err(io)?;
            file.sync_all().map_err(io)?;
            drop(file);

            if let Err(error) = fs::rename(&temp_path, path) {
                #[cfg(windows)]
                {
                    if path.exists() {
                        fs::remove_file(path).map_err(io)?;
                        fs::rename(&temp_path, path).map_err(io)?;
                    } else {
                        return Err(io(error));
                    }
                }

                #[cfg(not(windows))]
                {
                    return Err(io(error));
                }
            }

            Ok(())
        })();

        if result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }

        result
    }
}

pub fn config_dir() -> Option<PathBuf> {
    BaseDirs::new().map(|dirs| dirs.home_dir().join(CONFIG_DIR_NAME))
}

pub fn config_file() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join(CONFIG_FILE_NAME))
}

pub fn load() -> LoadResult {
    let Some(path) = config_file() else {
        return LoadResult {
            config: Config::default(),
            warning: Some(ConfigError::NoHome.to_string()),
        };
    };

    if !path.exists() {
        return LoadResult {
            config: Config::default(),
            warning: None,
        };
    }

    match fs::read(&path) {
        Ok(bytes) => match serde_json::from_slice::<Config>(&bytes) {
            Ok(config) => LoadResult {
                config,
                warning: None,
            },
            Err(error) => LoadResult {
                config: Config::default(),
                warning: Some(format!(
                    "A configuração em {} está inválida e foi ignorada: {error}",
                    path.display()
                )),
            },
        },
        Err(error) => LoadResult {
            config: Config::default(),
            warning: Some(format!("Não foi possível ler {}: {error}", path.display())),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_legacy_flat_config() {
        let raw = r##"{
            "vendor_id": 3141,
            "product_id": 34049,
            "interface_number": 1,
            "usage_page": 65280,
            "effect": 13,
            "bright": 2,
            "r": 255,
            "g": 87,
            "b": 51
        }"##;

        let config: Config = serde_json::from_str(raw).unwrap();
        let device = config.device().unwrap();
        let state = config.keyboard_state();

        assert_eq!(device.vendor_id, 0x0C45);
        assert_eq!(device.product_id, 0x8501);
        assert_eq!(device.interface_number, Some(1));
        assert_eq!(state.effect, Effect::Rainbow);
        assert_eq!(state.brightness, 2);
        assert_eq!(state.color, Rgb::new(255, 87, 51));
    }

    #[test]
    fn invalid_saved_values_fall_back_safely() {
        let config = Config {
            effect: Some(0xEE),
            bright: Some(200),
            r: Some(1),
            ..Config::default()
        };
        let state = config.keyboard_state();

        assert_eq!(state.effect, Effect::Static);
        assert_eq!(state.brightness, MAX_BRIGHTNESS);
        assert_eq!(state.color, Rgb::new(1, 255, 255));
    }

    #[test]
    fn invalid_device_is_ignored() {
        let config = Config {
            vendor_id: Some(0),
            product_id: Some(0x8501),
            ..Config::default()
        };
        assert!(config.device().is_none());
    }

    #[test]
    fn save_is_atomic_and_round_trips() {
        let dir = std::env::temp_dir().join(format!(
            "huskynext-config-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("config.json");

        let mut config = Config::default();
        config.set_device(DeviceConfig {
            vendor_id: 0x0C45,
            product_id: 0x8501,
            interface_number: Some(1),
            usage_page: Some(0xFF1C),
        });
        config.set_state(KeyboardState {
            effect: Effect::Rainbow,
            brightness: 1,
            color: Rgb::new(1, 2, 3),
        });
        config.save_to(&path).unwrap();

        let bytes = fs::read(&path).unwrap();
        let loaded: Config = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(loaded.device(), config.device());
        assert_eq!(loaded.keyboard_state(), config.keyboard_state());

        let leftovers = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);

        let _ = fs::remove_dir_all(&dir);
    }
}
