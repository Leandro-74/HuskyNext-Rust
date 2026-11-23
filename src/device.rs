use std::collections::BTreeMap;
use std::ffi::CString;
#[cfg(target_os = "linux")]
use std::path::PathBuf;

use hidapi::{HidApi, HidError};

use crate::config::DeviceConfig;
use crate::protocol::{KeyboardState, REPORT_ID, REPORT_LEN};

pub const VENDOR_USAGE_PAGE_MIN: u16 = 0xFF00;
pub const VENDOR_USAGE_PAGE_MAX: u16 = 0xFFFF;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceCandidate {
    pub path: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub interface_number: i32,
    pub usage_page: u16,
    pub usage: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
    pub vendor_specific: bool,
}

impl DeviceCandidate {
    pub fn display_name(&self) -> String {
        let product = self
            .product
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("Dispositivo HID");
        let manufacturer = self
            .manufacturer
            .as_deref()
            .filter(|value| !value.trim().is_empty());

        match manufacturer {
            Some(manufacturer) => format!("{manufacturer} {product}"),
            None => product.to_string(),
        }
    }

    pub fn ids(&self) -> String {
        format!(
            "{:04X}:{:04X} · interface {} · usage page 0x{:04X}",
            self.vendor_id, self.product_id, self.interface_number, self.usage_page
        )
    }

    pub fn device_config(&self) -> DeviceConfig {
        DeviceConfig {
            vendor_id: self.vendor_id,
            product_id: self.product_id,
            interface_number: Some(self.interface_number),
            usage_page: Some(self.usage_page),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("Não foi possível inicializar o acesso HID: {0}")]
    Initialization(String),
    #[error(
        "O teclado configurado não foi encontrado. Verifique a conexão ou detecte os dispositivos novamente."
    )]
    NotFound,
    #[error(
        "O teclado configurado {vendor_id:04X}:{product_id:04X} não foi encontrado. Use --list para inspecionar os dispositivos HID."
    )]
    SavedDeviceNotFound { vendor_id: u16, product_id: u16 },
    #[error("Sem permissão para acessar o dispositivo {vendor_id:04X}:{product_id:04X}. {details}")]
    Permission {
        vendor_id: u16,
        product_id: u16,
        details: String,
    },
    #[error("Falha ao abrir o dispositivo HID: {0}")]
    Open(String),
    #[error("Falha ao enviar o relatório HID: {0}")]
    Write(String),
    #[error("Envio incompleto: apenas {sent} de {expected} bytes foram escritos.")]
    Incomplete { sent: usize, expected: usize },
    #[error("Relatório inválido: esperado Report ID 0x{expected:02X}, encontrado 0x{found:02X}.")]
    InvalidReport { expected: u8, found: u8 },
    #[error("O caminho do dispositivo HID contém um byte inválido.")]
    InvalidPath,
}

impl DeviceError {
    pub fn user_hint(&self) -> Option<String> {
        match self {
            Self::Permission {
                vendor_id,
                product_id,
                ..
            } => Some(permission_hint(*vendor_id, *product_id)),
            _ => None,
        }
    }

    /// Returns true when the error may mean that the cached device path is out
    /// of date and a new discovery could fix it.
    pub fn is_stale(&self) -> bool {
        match self {
            Self::NotFound => true,
            Self::Open(message) | Self::Write(message) => {
                let lower = message.to_lowercase();
                lower.contains("no such file")
                    || lower.contains("no such device")
                    || lower.contains("device not found")
                    || lower.contains("broken pipe")
            }
            _ => false,
        }
    }
}

pub fn permission_hint(vendor_id: u16, product_id: u16) -> String {
    format!(
        "No Linux, crie /etc/udev/rules.d/99-huskynext.rules com:\n\
         SUBSYSTEM==\"usb\", ATTRS{{idVendor}}==\"{vendor_id:04x}\", ATTRS{{idProduct}}==\"{product_id:04x}\", TAG+=\"uaccess\"\n\
         SUBSYSTEM==\"hidraw\", ATTRS{{idVendor}}==\"{vendor_id:04x}\", ATTRS{{idProduct}}==\"{product_id:04x}\", TAG+=\"uaccess\"\n\
         Depois execute: sudo udevadm control --reload-rules && sudo udevadm trigger\n\
         Por fim, reconecte o teclado. O TAG uaccess libera o acesso apenas para o usuário da sessão ativa."
    )
}

pub fn discover() -> Result<Vec<DeviceCandidate>, DeviceError> {
    let api = HidApi::new().map_err(|error| DeviceError::Initialization(error.to_string()))?;
    let descriptors = sysfs_descriptors();

    let mut groups: BTreeMap<(String, i32, u16, u16), DeviceCandidate> = BTreeMap::new();

    for info in api.device_list() {
        let path = info.path().to_string_lossy().into_owned();
        let vendor_id = info.vendor_id();
        let product_id = info.product_id();
        let interface_number = info.interface_number();
        let usage_page = info.usage_page();
        let usage = info.usage();

        let vendor_specific = is_vendor_usage_page(usage_page)
            || descriptors
                .get(&path)
                .is_some_and(|descriptor| descriptor_has_vendor_usage_page(descriptor));

        let candidate = DeviceCandidate {
            path: path.clone(),
            vendor_id,
            product_id,
            interface_number,
            usage_page,
            usage,
            manufacturer: clean_string(info.manufacturer_string()),
            product: clean_string(info.product_string()),
            serial_number: clean_string(info.serial_number()),
            vendor_specific,
        };

        let key = (path, interface_number, vendor_id, product_id);
        match groups.get(&key) {
            Some(existing) if existing.vendor_specific && !candidate.vendor_specific => {}
            Some(existing)
                if existing.vendor_specific == candidate.vendor_specific
                    && existing.usage_page > candidate.usage_page => {}
            _ => {
                groups.insert(key, candidate);
            }
        }
    }

    let mut candidates: Vec<_> = groups.into_values().collect();
    candidates.sort_by(|a, b| {
        b.vendor_specific
            .cmp(&a.vendor_specific)
            .then_with(|| a.display_name().cmp(&b.display_name()))
            .then_with(|| a.vendor_id.cmp(&b.vendor_id))
            .then_with(|| a.product_id.cmp(&b.product_id))
            .then_with(|| a.interface_number.cmp(&b.interface_number))
            .then_with(|| a.path.cmp(&b.path))
    });

    Ok(candidates)
}

pub fn is_vendor_usage_page(usage_page: u16) -> bool {
    (VENDOR_USAGE_PAGE_MIN..=VENDOR_USAGE_PAGE_MAX).contains(&usage_page)
}

pub fn find_candidate(
    candidates: &[DeviceCandidate],
    config: DeviceConfig,
) -> Option<&DeviceCandidate> {
    let mut pool: Vec<&DeviceCandidate> = candidates
        .iter()
        .filter(|candidate| {
            candidate.vendor_id == config.vendor_id && candidate.product_id == config.product_id
        })
        .collect();

    if pool.is_empty() {
        return None;
    }

    if let Some(interface_number) = config.interface_number {
        let filtered: Vec<_> = pool
            .iter()
            .copied()
            .filter(|candidate| candidate.interface_number == interface_number)
            .collect();

        if !filtered.is_empty() {
            pool = filtered;
        }
    }

    if let Some(usage_page) = config.usage_page.filter(|value| *value != 0) {
        let filtered: Vec<_> = pool
            .iter()
            .copied()
            .filter(|candidate| candidate.usage_page == usage_page)
            .collect();

        if !filtered.is_empty() {
            pool = filtered;
        }
    }

    pool.sort_by_key(|candidate| !candidate.vendor_specific);
    pool.first().copied()
}

pub fn default_candidate(candidates: &[DeviceCandidate]) -> Option<&DeviceCandidate> {
    candidates
        .iter()
        .find(|candidate| {
            candidate.vendor_specific
                && (candidate
                    .manufacturer
                    .as_deref()
                    .is_some_and(|value| value.to_lowercase().contains("sonix"))
                    || candidate
                        .product
                        .as_deref()
                        .is_some_and(|value| value.to_lowercase().contains("sled")))
        })
        .or_else(|| {
            candidates
                .iter()
                .find(|candidate| candidate.vendor_specific)
        })
}

pub fn select_for_test(
    candidates: &[DeviceCandidate],
    saved: Option<DeviceConfig>,
) -> Result<&DeviceCandidate, DeviceError> {
    match saved {
        Some(config) => {
            find_candidate(candidates, config).ok_or(DeviceError::SavedDeviceNotFound {
                vendor_id: config.vendor_id,
                product_id: config.product_id,
            })
        }
        None => default_candidate(candidates).ok_or(DeviceError::NotFound),
    }
}

pub fn write_to_candidate(
    candidate: &DeviceCandidate,
    report: &[u8],
) -> Result<usize, DeviceError> {
    if report.len() != REPORT_LEN {
        return Err(DeviceError::Incomplete {
            sent: report.len(),
            expected: REPORT_LEN,
        });
    }

    if report[0] != REPORT_ID {
        return Err(DeviceError::InvalidReport {
            expected: REPORT_ID,
            found: report[0],
        });
    }

    let api = HidApi::new().map_err(|error| DeviceError::Initialization(error.to_string()))?;
    let path = CString::new(candidate.path.as_bytes()).map_err(|_| DeviceError::InvalidPath)?;

    let device = api
        .open_path(&path)
        .map_err(|error| map_open_error(error, candidate))?;
    let sent = device
        .write(report)
        .map_err(|error| map_write_error(error, candidate))?;

    if sent != report.len() {
        return Err(DeviceError::Incomplete {
            sent,
            expected: report.len(),
        });
    }

    Ok(sent)
}

pub fn send_state_with_candidates(
    candidates: &[DeviceCandidate],
    config: DeviceConfig,
    state: KeyboardState,
) -> Result<(usize, DeviceCandidate), DeviceError> {
    let candidate = find_candidate(candidates, config).ok_or(DeviceError::NotFound)?;
    let sent = write_to_candidate(candidate, &state.report())?;
    Ok((sent, candidate.clone()))
}

fn map_open_error(error: HidError, candidate: &DeviceCandidate) -> DeviceError {
    let message = error.to_string();
    let lower = message.to_lowercase();

    if lower.contains("permission denied")
        || lower.contains("access denied")
        || lower.contains("os error 13")
        || lower.contains("os error 1")
    {
        DeviceError::Permission {
            vendor_id: candidate.vendor_id,
            product_id: candidate.product_id,
            details: message,
        }
    } else {
        DeviceError::Open(message)
    }
}

fn map_write_error(error: HidError, candidate: &DeviceCandidate) -> DeviceError {
    let message = error.to_string();
    let lower = message.to_lowercase();

    if lower.contains("permission denied") || lower.contains("os error 13") {
        DeviceError::Permission {
            vendor_id: candidate.vendor_id,
            product_id: candidate.product_id,
            details: message,
        }
    } else {
        DeviceError::Write(message)
    }
}

fn clean_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn sysfs_descriptors() -> BTreeMap<String, Vec<u8>> {
    let mut descriptors = BTreeMap::new();

    #[cfg(target_os = "linux")]
    {
        let root = PathBuf::from("/sys/class/hidraw");
        let Ok(entries) = std::fs::read_dir(root) else {
            return descriptors;
        };

        for entry in entries.flatten() {
            let hidraw_name = entry.file_name().to_string_lossy().into_owned();
            if !hidraw_name.starts_with("hidraw") {
                continue;
            }

            let Ok(descriptor) = std::fs::read(entry.path().join("device/report_descriptor"))
            else {
                continue;
            };

            descriptors.insert(format!("/dev/{hidraw_name}"), descriptor.clone());

            if let Ok(device_path) = std::fs::canonicalize(entry.path().join("device"))
                && let Some(interface) = device_path.parent().and_then(|parent| parent.file_name())
            {
                descriptors
                    .entry(interface.to_string_lossy().into_owned())
                    .or_insert(descriptor);
            }
        }
    }

    descriptors
}

pub fn descriptor_has_vendor_usage_page(descriptor: &[u8]) -> bool {
    let mut index = 0;

    while index < descriptor.len() {
        let prefix = descriptor[index];

        if prefix == 0xFE {
            let Some(length) = descriptor.get(index + 1) else {
                break;
            };
            index += 3 + usize::from(*length);
            continue;
        }

        let size = match prefix & 0x03 {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 4,
        };
        let tag = prefix & 0xFC;

        if tag == 0x04 && index + 1 + size <= descriptor.len() {
            let bytes = &descriptor[index + 1..index + 1 + size];
            let mut value = 0u32;

            for (shift, byte) in bytes.iter().enumerate() {
                value |= u32::from(*byte) << (shift * 8);
            }

            if (u32::from(VENDOR_USAGE_PAGE_MIN)..=u32::from(VENDOR_USAGE_PAGE_MAX))
                .contains(&value)
            {
                return true;
            }
        }

        index += 1 + size;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(vendor_specific: bool, interface_number: i32, usage_page: u16) -> DeviceCandidate {
        DeviceCandidate {
            path: format!("/dev/hidraw{interface_number}"),
            vendor_id: 0x0C45,
            product_id: 0x8501,
            interface_number,
            usage_page,
            usage: 0x92,
            manufacturer: Some("SONiX".into()),
            product: Some("USB DEVICE".into()),
            serial_number: None,
            vendor_specific,
        }
    }

    #[test]
    fn detects_vendor_page_in_real_descriptor() {
        let descriptor = [
            0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x85, 0x01, 0x05, 0x07, 0x19, 0x04, 0x29, 0x70,
            0x15, 0x00, 0x25, 0x01, 0x75, 0x01, 0x95, 0x78, 0x81, 0x02, 0xC0, 0x06, 0x1C, 0xFF,
            0x09, 0x92, 0xA1, 0x01, 0x85, 0x04, 0x19, 0x00, 0x2A, 0xFF, 0x00, 0x15, 0x00, 0x26,
            0xFF, 0x00, 0x75, 0x08, 0x95, 0x3F, 0x91, 0x00, 0xC0,
        ];
        assert!(descriptor_has_vendor_usage_page(&descriptor));
    }

    #[test]
    fn rejects_keyboard_only_descriptor() {
        let descriptor = [0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0xC0];
        assert!(!descriptor_has_vendor_usage_page(&descriptor));
    }

    #[test]
    fn skips_long_descriptor_items() {
        let descriptor = [0xFE, 0x02, 0x00, 0xAA, 0xBB, 0x06, 0x1C, 0xFF];
        assert!(descriptor_has_vendor_usage_page(&descriptor));
    }

    #[test]
    fn stale_usage_page_does_not_block_match() {
        let candidates = [candidate(true, 1, 0xFF1C)];
        let config = DeviceConfig {
            vendor_id: 0x0C45,
            product_id: 0x8501,
            interface_number: Some(1),
            usage_page: Some(0xFF00),
        };
        assert!(find_candidate(&candidates, config).is_some());
    }

    #[test]
    fn stale_interface_falls_back_to_vendor_specific() {
        let candidates = [candidate(false, 0, 0x0001), candidate(true, 2, 0xFF1C)];
        let config = DeviceConfig {
            vendor_id: 0x0C45,
            product_id: 0x8501,
            interface_number: Some(1),
            usage_page: None,
        };
        let found = find_candidate(&candidates, config).unwrap();
        assert_eq!(found.interface_number, 2);
        assert!(found.vendor_specific);
    }

    #[test]
    fn vendor_specific_is_preferred() {
        let candidates = [candidate(false, 1, 0x0001), candidate(true, 1, 0xFF1C)];
        let config = DeviceConfig {
            vendor_id: 0x0C45,
            product_id: 0x8501,
            interface_number: Some(1),
            usage_page: Some(0xFF1C),
        };
        let found = find_candidate(&candidates, config).unwrap();
        assert!(found.vendor_specific);
    }

    #[test]
    fn select_for_test_uses_saved_device() {
        let candidates = [candidate(false, 1, 0x0001), candidate(true, 2, 0xFF1C)];
        let saved = DeviceConfig {
            vendor_id: 0x0C45,
            product_id: 0x8501,
            interface_number: Some(1),
            usage_page: Some(0x0001),
        };
        let found = select_for_test(&candidates, Some(saved)).unwrap();
        assert_eq!(found.interface_number, 1);
        assert!(!found.vendor_specific);
    }

    #[test]
    fn select_for_test_fails_when_saved_device_is_missing() {
        let candidates = [candidate(true, 1, 0xFF1C)];
        let saved = DeviceConfig {
            vendor_id: 0x1234,
            product_id: 0x5678,
            interface_number: Some(1),
            usage_page: Some(0xFF00),
        };
        match select_for_test(&candidates, Some(saved)) {
            Err(DeviceError::SavedDeviceNotFound {
                vendor_id,
                product_id,
            }) => {
                assert_eq!(vendor_id, 0x1234);
                assert_eq!(product_id, 0x5678);
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn select_for_test_prefers_sonix_or_sled_without_config() {
        let mut other = candidate(true, 1, 0xFF1C);
        other.manufacturer = Some("Other".into());
        other.product = Some("Keyboard".into());
        let sonix = candidate(true, 2, 0xFF1C);
        let candidates = [other, sonix];
        let found = select_for_test(&candidates, None).unwrap();
        assert_eq!(found.manufacturer.as_deref(), Some("SONiX"));
    }

    #[test]
    fn select_for_test_without_candidates_returns_not_found() {
        assert!(matches!(
            select_for_test(&[], None),
            Err(DeviceError::NotFound)
        ));
    }

    #[test]
    fn default_candidate_prefers_vendor_specific() {
        let candidates = [candidate(false, 1, 0x0001), candidate(true, 2, 0xFF1C)];
        let found = default_candidate(&candidates).unwrap();
        assert!(found.vendor_specific);
        assert_eq!(found.interface_number, 2);
    }

    #[test]
    fn send_state_with_candidates_fails_without_candidates() {
        let config = DeviceConfig {
            vendor_id: 0x0C45,
            product_id: 0x8501,
            interface_number: Some(1),
            usage_page: Some(0xFF1C),
        };
        assert!(matches!(
            send_state_with_candidates(&[], config, KeyboardState::default()),
            Err(DeviceError::NotFound)
        ));
    }

    #[test]
    fn write_to_candidate_rejects_short_report_without_opening_device() {
        let candidate = candidate(true, 1, 0xFF1C);
        let report = [0x04, 0x00, 0x00];
        match write_to_candidate(&candidate, &report) {
            Err(DeviceError::Incomplete { sent, expected }) => {
                assert_eq!(sent, report.len());
                assert_eq!(expected, REPORT_LEN);
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn write_to_candidate_rejects_invalid_report_id_without_opening_device() {
        let candidate = candidate(true, 1, 0xFF1C);
        let mut report = [0u8; REPORT_LEN];
        report[0] = 0x01;
        match write_to_candidate(&candidate, &report) {
            Err(DeviceError::InvalidReport { expected, found }) => {
                assert_eq!(expected, REPORT_ID);
                assert_eq!(found, 0x01);
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn user_hint_mentions_uaccess_for_permission_errors() {
        let hint = DeviceError::Permission {
            vendor_id: 0x0C45,
            product_id: 0x8501,
            details: "permission denied".into(),
        }
        .user_hint()
        .unwrap();
        assert!(hint.contains("uaccess"));
    }

    #[test]
    fn user_hint_is_absent_for_not_found() {
        assert!(DeviceError::NotFound.user_hint().is_none());
    }

    #[test]
    fn permission_hint_includes_formatted_ids() {
        let hint = permission_hint(0x0C45, 0x8501);
        assert!(hint.contains("0c45"));
        assert!(hint.contains("8501"));
        assert!(hint.contains("/etc/udev/rules.d/99-huskynext.rules"));
    }

    #[test]
    fn vendor_usage_page_boundaries() {
        assert!(!is_vendor_usage_page(0xFEFF));
        assert!(is_vendor_usage_page(0xFF00));
        assert!(is_vendor_usage_page(0xFFFF));
        assert!(!is_vendor_usage_page(0x0000));
    }

    #[test]
    fn clean_string_trims_and_drops_empty_values() {
        assert_eq!(clean_string(None), None);
        assert_eq!(clean_string(Some("")), None);
        assert_eq!(clean_string(Some("   ")), None);
        assert_eq!(clean_string(Some("  SONiX  ")), Some("SONiX".to_string()));
    }

    #[test]
    fn stale_detection_matches_outdated_paths() {
        assert!(DeviceError::NotFound.is_stale());
        assert!(DeviceError::Open("No such file or directory".into()).is_stale());
        assert!(DeviceError::Write("broken pipe".into()).is_stale());
        assert!(
            !DeviceError::Permission {
                vendor_id: 0x0C45,
                product_id: 0x8501,
                details: "permission denied".into(),
            }
            .is_stale()
        );
        assert!(!DeviceError::Open("permission denied".into()).is_stale());
    }
}
