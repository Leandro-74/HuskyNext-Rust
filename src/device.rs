use std::collections::BTreeMap;
use std::ffi::CString;
#[cfg(target_os = "linux")]
use std::path::PathBuf;

use hidapi::{HidApi, HidError};

use crate::config::DeviceConfig;
use crate::protocol::{KeyboardState, REPORT_LEN};

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

pub fn send_state(
    config: DeviceConfig,
    state: KeyboardState,
) -> Result<(usize, DeviceCandidate), DeviceError> {
    let candidates = discover()?;
    let candidate = find_candidate(&candidates, config).ok_or(DeviceError::NotFound)?;
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
}
