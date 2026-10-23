use std::time::{Duration, Instant};

use iced::widget::{
    Container, Space, button, canvas, column, container, image, row, scrollable, slider, text,
    text_input,
};
use iced::widget::{
    button::{Status as ButtonStatus, Style as ButtonStyle},
    container::Style as ContainerStyle,
    image::Handle as ImageHandle,
    slider::{
        Handle as SliderHandle, HandleShape, Rail, Status as SliderStatus, Style as SliderStyle,
    },
    text_input::{Status as InputStatus, Style as InputStyle},
};
use iced::{
    Alignment, Background, Border, Color, Element, Font, Length, Point, Rectangle, Shadow, Size,
    Subscription, Theme, Vector, mouse, window,
};

use crate::color::Hsv;
use crate::config::{self, Config, DeviceConfig};
use crate::device::{self, DeviceCandidate, DeviceError};
use crate::protocol::{
    CONNECTION_TEST_COLOR, Effect, KeyboardState, MAX_BRIGHTNESS, Rgb, brightness_percent,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const C_BG: Color = Color::from_rgb8(0x0D, 0x0F, 0x14);
const C_SIDEBAR: Color = Color::from_rgb8(0x0A, 0x0C, 0x11);
const C_SURFACE: Color = Color::from_rgb8(0x16, 0x1A, 0x23);
const C_SURFACE_ALT: Color = Color::from_rgb8(0x1D, 0x23, 0x30);
const C_INPUT: Color = Color::from_rgb8(0x11, 0x14, 0x1C);
const C_TRACK: Color = Color::from_rgb8(0x2A, 0x31, 0x42);
const C_BORDER: Color = Color::from_rgb8(0x2A, 0x31, 0x42);
const C_BORDER_LIGHT: Color = Color::from_rgb8(0x3A, 0x44, 0x5A);
const C_TEXT: Color = Color::from_rgb8(0xEE, 0xF1, 0xF7);
const C_MUTED: Color = Color::from_rgb8(0x9A, 0xA4, 0xB2);
const C_ACCENT: Color = Color::from_rgb8(0x88, 0x35, 0xE9);
const C_ACCENT_HOVER: Color = Color::from_rgb8(0x91, 0x3F, 0xF0);
const C_ACCENT_PRESSED: Color = Color::from_rgb8(0x6C, 0x29, 0xBA);
const C_SUCCESS: Color = Color::from_rgb8(0x39, 0xD9, 0x8A);
const C_WARNING: Color = Color::from_rgb8(0xF2, 0xB8, 0x4B);
const C_ERROR: Color = Color::from_rgb8(0xFF, 0x6B, 0x6B);
const C_INFO: Color = Color::from_rgb8(0x6E, 0xA8, 0xFE);

const PICKER_SIZE: f32 = 200.0;
const HUE_HEIGHT: f32 = 14.0;
const HUE_GAP: f32 = 16.0;
const PICKER_HEIGHT: f32 = PICKER_SIZE + HUE_GAP + HUE_HEIGHT;
const APPLY_INTERVAL: Duration = Duration::from_millis(120);
const SAVE_DELAY: Duration = Duration::from_millis(450);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Lighting,
    Connection,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NoticeKind {
    Info,
    Success,
    Warning,
    Error,
}

impl NoticeKind {
    fn color(self) -> Color {
        match self {
            Self::Info => C_INFO,
            Self::Success => C_SUCCESS,
            Self::Warning => C_WARNING,
            Self::Error => C_ERROR,
        }
    }
}

#[derive(Debug, Clone)]
struct Notice {
    kind: NoticeKind,
    title: String,
    detail: Option<String>,
}

impl Notice {
    fn new(kind: NoticeKind, title: impl Into<String>) -> Self {
        Self {
            kind,
            title: title.into(),
            detail: None,
        }
    }

    fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    fn info(title: impl Into<String>) -> Self {
        Self::new(NoticeKind::Info, title)
    }

    fn success(title: impl Into<String>) -> Self {
        Self::new(NoticeKind::Success, title)
    }

    fn warning(title: impl Into<String>) -> Self {
        Self::new(NoticeKind::Warning, title)
    }

    fn error(title: impl Into<String>) -> Self {
        Self::new(NoticeKind::Error, title)
    }
}

#[derive(Debug, Clone)]
enum Message {
    Navigate(Page),
    EffectSelected(Effect),
    BrightnessChanged(u8),
    RedChanged(u8),
    GreenChanged(u8),
    BlueChanged(u8),
    HexChanged(String),
    HexSubmitted,
    PresetColor(Rgb),
    PickerColor(Hsv),
    PickerReleased,
    Apply,
    AutoApplyTick,
    ResetDefaults,
    DismissNotice,
    Rescan,
    ShowAllToggled(bool),
    TestDevice(String),
    UseDevice(String),
    ForgetDevice,
    ManualVidChanged(String),
    ManualPidChanged(String),
    ManualIfaceChanged(String),
    ManualUsageChanged(String),
    ManualConnect,
}

pub struct App {
    page: Page,
    config: Config,
    state: KeyboardState,
    picker_hsv: Hsv,
    picker_active: bool,
    picker_changed: bool,
    hex_input: String,
    hex_error: Option<String>,
    candidates: Vec<DeviceCandidate>,
    selected_candidate: Option<String>,
    show_all_devices: bool,
    dirty: bool,
    auto_apply_paused: bool,
    pending_save: bool,
    save_ready_at: Option<Instant>,
    notice: Option<Notice>,
    manual_vid: String,
    manual_pid: String,
    manual_iface: String,
    manual_usage: String,
    logo: ImageHandle,
}

impl App {
    pub fn new() -> Self {
        let loaded = config::load();
        let config = loaded.config;
        let state = config.keyboard_state();

        let mut notice = loaded.warning.map(Notice::warning);
        let candidates = match device::discover() {
            Ok(candidates) => candidates,
            Err(error) => {
                notice = Some(
                    Notice::error("Falha ao detectar dispositivos HID")
                        .with_detail(error.to_string()),
                );
                Vec::new()
            }
        };

        let saved_device = config.device();
        let selected_candidate = saved_device
            .and_then(|saved| device::find_candidate(&candidates, saved))
            .map(|candidate| candidate.path.clone());

        if let Some(saved) = saved_device
            && selected_candidate.is_none()
            && notice.is_none()
        {
            notice = Some(
                Notice::warning("Teclado configurado não encontrado").with_detail(format!(
                    "Procurei por {:04X}:{:04X}. Verifique se o teclado está conectado ou use a página Conexão.",
                    saved.vendor_id, saved.product_id
                )),
            );
        }

        let manual_vid = saved_device
            .map(|saved| format!("{:04X}", saved.vendor_id))
            .unwrap_or_default();
        let manual_pid = saved_device
            .map(|saved| format!("{:04X}", saved.product_id))
            .unwrap_or_default();
        let manual_iface = saved_device
            .and_then(|saved| saved.interface_number)
            .map(|iface| iface.to_string())
            .unwrap_or_default();
        let manual_usage = saved_device
            .and_then(|saved| saved.usage_page)
            .map(|page| format!("{page:04X}"))
            .unwrap_or_default();

        let page = if saved_device.is_some() {
            Page::Lighting
        } else {
            Page::Connection
        };

        let show_all_devices = !candidates.iter().any(|candidate| candidate.vendor_specific);

        Self {
            page,
            config,
            state,
            picker_hsv: Hsv::from_rgb(state.color),
            picker_active: false,
            picker_changed: false,
            hex_input: state.color.to_hex(),
            hex_error: None,
            candidates,
            selected_candidate,
            show_all_devices,
            dirty: false,
            auto_apply_paused: false,
            pending_save: false,
            save_ready_at: None,
            notice,
            manual_vid,
            manual_pid,
            manual_iface,
            manual_usage,
            logo: ImageHandle::from_bytes(&include_bytes!("../assets/logo.png")[..]),
        }
    }

    fn current_candidate(&self) -> Option<&DeviceCandidate> {
        self.config
            .device()
            .and_then(|saved| device::find_candidate(&self.candidates, saved))
    }

    fn remember_candidate(&mut self, candidate: DeviceCandidate) {
        if let Some(existing) = self
            .candidates
            .iter_mut()
            .find(|existing| existing.path == candidate.path)
        {
            *existing = candidate;
        } else {
            self.candidates.push(candidate);
        }
    }

    fn is_in_use(&self, candidate: &DeviceCandidate) -> bool {
        let Some(saved) = self.config.device() else {
            return false;
        };

        saved.vendor_id == candidate.vendor_id
            && saved.product_id == candidate.product_id
            && saved
                .interface_number
                .is_none_or(|iface| iface == candidate.interface_number)
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.auto_apply_paused = false;

        if self.config.device().is_some() {
            self.pending_save = true;
            self.save_ready_at = Some(Instant::now() + SAVE_DELAY);
        } else {
            self.pending_save = false;
            self.save_ready_at = None;
        }
    }

    fn set_color(&mut self, color: Rgb) {
        self.state.color = color;
        self.hex_input = color.to_hex();
        self.hex_error = None;
        self.picker_hsv = Hsv::from_rgb_with_hue(color, self.picker_hsv.h);
        self.mark_dirty();
    }

    fn set_picker_color(&mut self, hsv: Hsv) {
        self.picker_hsv = hsv;
        self.state.color = hsv.to_rgb();
        self.hex_input = self.state.color.to_hex();
        self.hex_error = None;
    }

    fn sync_hex_input(&mut self) -> bool {
        match Rgb::from_hex(&self.hex_input) {
            Ok(color) => {
                self.state.color = color;
                self.picker_hsv = Hsv::from_rgb_with_hue(color, self.picker_hsv.h);
                self.hex_input = color.to_hex();
                self.hex_error = None;
                true
            }
            Err(error) => {
                self.hex_error = Some(error.to_string());
                false
            }
        }
    }

    fn apply(&mut self, manual: bool) {
        if manual && !self.sync_hex_input() {
            self.notice = Some(
                Notice::error("Cor inválida").with_detail(
                    self.hex_error
                        .clone()
                        .unwrap_or_else(|| "Use o formato RRGGBB.".to_string()),
                ),
            );
            return;
        }

        let Some(saved) = self.config.device() else {
            if manual {
                self.page = Page::Connection;
                self.notice = Some(
                    Notice::warning("Nenhum teclado configurado")
                        .with_detail("Escolha o dispositivo de iluminação na página Conexão."),
                );
            }
            return;
        };

        match device::send_state(saved, self.state) {
            Ok((sent, candidate)) => {
                self.dirty = false;
                self.auto_apply_paused = false;
                self.selected_candidate = Some(candidate.path.clone());
                self.config.set_device(candidate.device_config());
                self.config.set_state(self.state);
                self.remember_candidate(candidate.clone());

                if manual {
                    self.pending_save = false;
                    self.save_ready_at = None;
                    let save_error = self.config.save().err();
                    self.notice = Some(match save_error {
                        Some(error) => Notice::warning("Iluminação enviada, mas não foi salva")
                            .with_detail(format!(
                                "O teclado recebeu {sent} bytes, porém a configuração não pôde ser gravada: {error}"
                            )),
                        None => Notice::success("Iluminação aplicada").with_detail(format!(
                            "{sent} bytes enviados para {}.",
                            candidate.display_name()
                        )),
                    });
                } else {
                    self.pending_save = true;
                    if self.save_ready_at.is_none() {
                        self.save_ready_at = Some(Instant::now() + SAVE_DELAY);
                    }
                    if self
                        .notice
                        .as_ref()
                        .is_some_and(|notice| notice.kind == NoticeKind::Error)
                    {
                        self.notice = None;
                    }
                }
            }
            Err(error) => {
                self.dirty = true;
                if manual {
                    self.notice = Some(device_error_notice(
                        "Não foi possível aplicar a iluminação",
                        &error,
                    ));
                    if matches!(
                        error,
                        DeviceError::NotFound | DeviceError::Permission { .. }
                    ) {
                        self.page = Page::Connection;
                    }
                } else {
                    self.auto_apply_paused = true;
                    self.pending_save = false;
                    self.save_ready_at = None;
                    self.notice = Some(device_error_notice(
                        "Falha ao aplicar automaticamente",
                        &error,
                    ));
                }
            }
        }
    }

    fn auto_apply_tick(&mut self) {
        if self.dirty
            && !self.auto_apply_paused
            && !self.picker_active
            && self.config.device().is_some()
        {
            self.apply(false);
        }

        self.flush_pending_save();
    }

    fn flush_pending_save(&mut self) {
        if !self.pending_save || self.dirty {
            return;
        }

        let Some(deadline) = self.save_ready_at else {
            return;
        };

        if Instant::now() < deadline {
            return;
        }

        self.pending_save = false;
        self.save_ready_at = None;

        if let Err(error) = self.config.save() {
            self.notice = Some(
                Notice::warning("Configuração não salva").with_detail(format!(
                    "A iluminação foi aplicada, mas o arquivo não pôde ser gravado: {error}"
                )),
            );
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let needs_tick = (self.dirty
            && !self.auto_apply_paused
            && !self.picker_active
            && self.config.device().is_some())
            || self.pending_save;

        if needs_tick {
            iced::time::every(APPLY_INTERVAL).map(|_| Message::AutoApplyTick)
        } else {
            Subscription::none()
        }
    }

    fn rescan(&mut self) {
        match device::discover() {
            Ok(candidates) => {
                self.candidates = candidates;
                if !self.show_all_devices
                    && !self
                        .candidates
                        .iter()
                        .any(|candidate| candidate.vendor_specific)
                {
                    self.show_all_devices = true;
                }

                let saved = self.config.device();
                self.selected_candidate = saved
                    .and_then(|saved| device::find_candidate(&self.candidates, saved))
                    .map(|candidate| candidate.path.clone());

                self.notice = Some(match (saved, self.selected_candidate.as_ref()) {
                    (Some(_), Some(_)) => Notice::success("Dispositivos atualizados")
                        .with_detail("O teclado configurado foi encontrado."),
                    (Some(_), None) => Notice::warning("Teclado configurado não encontrado")
                        .with_detail("Outros dispositivos HID podem estar disponíveis na lista."),
                    (None, _) => Notice::info("Dispositivos atualizados").with_detail(format!(
                        "{} dispositivo(s) HID encontrado(s).",
                        self.candidates.len()
                    )),
                });
            }
            Err(error) => {
                self.notice = Some(
                    Notice::error("Falha ao detectar dispositivos").with_detail(error.to_string()),
                );
            }
        }
    }

    fn test_candidate(&mut self, path: &str) {
        let Some(candidate) = self
            .candidates
            .iter()
            .find(|candidate| candidate.path == path)
            .cloned()
        else {
            self.notice = Some(Notice::error("Dispositivo não está mais na lista"));
            return;
        };

        let test_state = KeyboardState {
            effect: Effect::Static,
            brightness: MAX_BRIGHTNESS,
            color: CONNECTION_TEST_COLOR,
        };

        match device::write_to_candidate(&candidate, &test_state.report()) {
            Ok(sent) => {
                self.selected_candidate = Some(candidate.path.clone());
                self.notice = Some(
                    Notice::success("Teste enviado")
                        .with_detail(format!(
                            "{sent} bytes enviados para {}. A luz ficou roxa ({})? Se sim, use “Usar este dispositivo”.",
                            candidate.display_name(),
                            CONNECTION_TEST_COLOR.to_hex()
                        )),
                );
            }
            Err(error) => {
                self.notice = Some(device_error_notice("Falha no teste", &error));
            }
        }
    }

    fn use_candidate(&mut self, path: &str) {
        let Some(candidate) = self
            .candidates
            .iter()
            .find(|candidate| candidate.path == path)
            .cloned()
        else {
            self.notice = Some(Notice::error("Dispositivo não está mais na lista"));
            return;
        };

        match device::write_to_candidate(&candidate, &self.state.report()) {
            Ok(sent) => {
                self.config.set_device(candidate.device_config());
                self.config.set_state(self.state);
                self.dirty = false;
                self.auto_apply_paused = false;
                self.pending_save = false;
                self.save_ready_at = None;
                self.selected_candidate = Some(candidate.path.clone());
                self.remember_candidate(candidate.clone());
                self.manual_vid = format!("{:04X}", candidate.vendor_id);
                self.manual_pid = format!("{:04X}", candidate.product_id);
                self.manual_iface = candidate.interface_number.to_string();
                self.manual_usage = format!("{:04X}", candidate.usage_page);
                self.page = Page::Lighting;

                let save_error = self.config.save().err();
                self.notice = Some(match save_error {
                    Some(error) => Notice::warning("Dispositivo conectado, mas não salvo")
                        .with_detail(format!(
                            "O teclado recebeu {sent} bytes, porém a configuração não pôde ser gravada: {error}"
                        )),
                    None => Notice::success("Teclado configurado").with_detail(format!(
                        "{} — {sent} bytes enviados.",
                        candidate.display_name()
                    )),
                });
            }
            Err(error) => {
                self.notice = Some(device_error_notice(
                    "Não foi possível usar este dispositivo",
                    &error,
                ));
            }
        }
    }

    fn forget_device(&mut self) {
        self.config.clear_device();
        self.selected_candidate = None;
        self.dirty = false;
        self.auto_apply_paused = false;
        self.pending_save = false;
        self.save_ready_at = None;

        match self.config.save() {
            Ok(()) => {
                self.page = Page::Connection;
                self.notice = Some(Notice::info("Dispositivo esquecido").with_detail(
                    "Escolha outro dispositivo na lista ou informe os IDs manualmente.",
                ));
            }
            Err(error) => {
                self.notice = Some(
                    Notice::warning("Dispositivo removido da sessão, mas não do arquivo")
                        .with_detail(error.to_string()),
                );
            }
        }
    }

    fn manual_connect(&mut self) {
        let Some(vendor_id) = parse_hex_u16(&self.manual_vid) else {
            self.notice = Some(
                Notice::error("Vendor ID inválido").with_detail("Use hexadecimal, como 0C45."),
            );
            return;
        };
        let Some(product_id) = parse_hex_u16(&self.manual_pid) else {
            self.notice = Some(
                Notice::error("Product ID inválido").with_detail("Use hexadecimal, como 8501."),
            );
            return;
        };
        if vendor_id == 0 || product_id == 0 {
            self.notice = Some(Notice::error("VID e PID devem ser diferentes de zero"));
            return;
        }

        let interface_number = match self.manual_iface.trim() {
            "" => None,
            value => match value.parse::<i32>() {
                Ok(value) if value >= 0 => Some(value),
                _ => {
                    self.notice =
                        Some(Notice::error("Interface inválida").with_detail(
                            "Informe um número inteiro não negativo ou deixe em branco.",
                        ));
                    return;
                }
            },
        };

        let usage_page = match self.manual_usage.trim() {
            "" => None,
            value => match parse_hex_u16(value) {
                Some(value) => Some(value),
                None => {
                    self.notice = Some(
                        Notice::error("Usage page inválido")
                            .with_detail("Use hexadecimal, como FF1C, ou deixe em branco."),
                    );
                    return;
                }
            },
        };

        let saved = DeviceConfig {
            vendor_id,
            product_id,
            interface_number,
            usage_page,
        };

        match device::find_candidate(&self.candidates, saved) {
            Some(candidate) => {
                let path = candidate.path.clone();
                self.use_candidate(&path);
            }
            None => {
                self.notice = Some(
                    Notice::error("Nenhum dispositivo correspondente")
                        .with_detail(format!(
                            "Não encontrei {:04X}:{:04X} na enumeração atual. Clique em “Detectar novamente” ou confira VID/PID com lsusb.",
                            vendor_id, product_id
                        )),
                );
            }
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Navigate(page) => self.page = page,
            Message::EffectSelected(effect) => {
                self.state.effect = effect;
                self.mark_dirty();
            }
            Message::BrightnessChanged(brightness) => {
                self.state.brightness = brightness.min(MAX_BRIGHTNESS);
                self.mark_dirty();
            }
            Message::RedChanged(value) => {
                let color = Rgb::new(value, self.state.color.g, self.state.color.b);
                self.set_color(color);
            }
            Message::GreenChanged(value) => {
                let color = Rgb::new(self.state.color.r, value, self.state.color.b);
                self.set_color(color);
            }
            Message::BlueChanged(value) => {
                let color = Rgb::new(self.state.color.r, self.state.color.g, value);
                self.set_color(color);
            }
            Message::HexChanged(value) => {
                self.hex_input = value;
                if let Ok(color) = Rgb::from_hex(&self.hex_input) {
                    self.state.color = color;
                    self.picker_hsv = Hsv::from_rgb_with_hue(color, self.picker_hsv.h);
                    self.hex_error = None;
                    self.mark_dirty();
                }
            }
            Message::HexSubmitted => {
                if self.sync_hex_input() {
                    self.mark_dirty();
                }
            }
            Message::PresetColor(color) => self.set_color(color),
            Message::PickerColor(hsv) => {
                self.picker_active = true;
                if hsv.to_rgb() != self.state.color {
                    self.picker_changed = true;
                }
                self.set_picker_color(hsv);
            }
            Message::PickerReleased => {
                if self.picker_active {
                    self.picker_active = false;
                    if self.picker_changed || self.dirty {
                        self.mark_dirty();
                        self.apply(false);
                    }
                    self.picker_changed = false;
                }
            }
            Message::Apply => {
                self.picker_active = false;
                self.apply(true);
            }
            Message::AutoApplyTick => self.auto_apply_tick(),
            Message::ResetDefaults => {
                self.state = KeyboardState::default();
                self.picker_hsv = Hsv::from_rgb(self.state.color);
                self.hex_input = self.state.color.to_hex();
                self.hex_error = None;
                self.mark_dirty();
                self.notice = Some(Notice::info("Padrão restaurado").with_detail(
                    if self.config.device().is_some() {
                        "A iluminação será sincronizada automaticamente."
                    } else {
                        "Configure o teclado na página Conexão para enviar."
                    },
                ));
            }
            Message::DismissNotice => self.notice = None,
            Message::Rescan => self.rescan(),
            Message::ShowAllToggled(show_all) => self.show_all_devices = show_all,
            Message::TestDevice(path) => self.test_candidate(&path),
            Message::UseDevice(path) => self.use_candidate(&path),
            Message::ForgetDevice => self.forget_device(),
            Message::ManualVidChanged(value) => self.manual_vid = value,
            Message::ManualPidChanged(value) => self.manual_pid = value,
            Message::ManualIfaceChanged(value) => self.manual_iface = value,
            Message::ManualUsageChanged(value) => self.manual_usage = value,
            Message::ManualConnect => self.manual_connect(),
        }
    }

    fn theme(&self) -> Theme {
        Theme::custom(
            "HuskyNext",
            iced::theme::Palette {
                background: C_BG,
                text: C_TEXT,
                primary: C_ACCENT,
                success: C_SUCCESS,
                warning: C_WARNING,
                danger: C_ERROR,
            },
        )
    }

    fn view(&self) -> Element<'_, Message> {
        let sidebar = self.sidebar();
        let content = match self.page {
            Page::Lighting => self.lighting_page(),
            Page::Connection => self.connection_page(),
            Page::About => self.about_page(),
        };

        row![
            sidebar,
            scrollable(
                container(content)
                    .width(Length::Fill)
                    .padding([28, 32])
                    .max_width(1040)
            )
            .width(Length::Fill)
            .height(Length::Fill),
        ]
        .height(Length::Fill)
        .into()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let brand = row![
            image(self.logo.clone()).width(44).height(44),
            column![
                text("HuskyNext").size(19).font(bold()),
                text("Husky Sled").size(12).color(C_MUTED),
            ]
            .spacing(1),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let (status_color, status_text) = if self.config.device().is_none() {
            (C_MUTED, "Não configurado")
        } else if self.current_candidate().is_some() {
            (C_SUCCESS, "Teclado encontrado")
        } else {
            (C_WARNING, "Teclado ausente")
        };

        let status = container(
            column![
                row![dot(status_color), text(status_text).size(13).color(C_TEXT),]
                    .spacing(8)
                    .align_y(Alignment::Center),
                text(
                    self.current_candidate()
                        .map(DeviceCandidate::display_name)
                        .unwrap_or_else(|| "Página Conexão para configurar".to_string())
                )
                .size(11)
                .color(C_MUTED),
            ]
            .spacing(4),
        )
        .padding(12)
        .width(Length::Fill)
        .style(sidebar_status_style);

        let nav = column![
            nav_button("Iluminação", Page::Lighting, self.page),
            nav_button("Conexão", Page::Connection, self.page),
            nav_button("Sobre", Page::About, self.page),
        ]
        .spacing(6);

        container(
            column![
                brand,
                status,
                nav,
                Space::new().height(Length::Fill),
                text(format!("v{VERSION} · Report ID 0x04"))
                    .size(10)
                    .color(C_MUTED),
            ]
            .spacing(22),
        )
        .padding(20)
        .width(Length::Fixed(224.0))
        .height(Length::Fill)
        .style(sidebar_style)
        .into()
    }

    fn lighting_page(&self) -> Element<'_, Message> {
        let header = self.page_header(
            "Iluminação",
            "Ajuste a cor, o brilho e o efeito — tudo é aplicado automaticamente.",
        );

        let mut content = column![header].spacing(18);
        if let Some(notice) = self.notice_view() {
            content = content.push(notice);
        }

        let picker = canvas::Canvas::new(ColorPicker {
            hsv: self.picker_hsv,
        })
        .width(Length::Fixed(PICKER_SIZE))
        .height(Length::Fixed(PICKER_HEIGHT));

        let picker_column = column![
            picker,
            text("Arraste para escolher; a cor é enviada ao soltar.")
                .size(10)
                .color(C_MUTED)
                .width(Length::Fixed(PICKER_SIZE)),
        ]
        .spacing(6)
        .width(Length::Fixed(PICKER_SIZE));

        let controls = column![
            row![
                container("")
                    .width(Length::Fixed(56.0))
                    .height(Length::Fixed(56.0))
                    .style(preview_style(iced_color(self.state.color))),
                column![
                    text_input("#RRGGBB", &self.hex_input)
                        .on_input(Message::HexChanged)
                        .on_submit(Message::HexSubmitted)
                        .padding(10)
                        .size(14)
                        .style(input_style(self.hex_error.is_some())),
                    text(self.hex_error.clone().unwrap_or_else(|| {
                        "Digite um HEX ou use o seletor e os canais.".to_string()
                    }))
                    .size(11)
                    .color(if self.hex_error.is_some() {
                        C_ERROR
                    } else {
                        C_MUTED
                    }),
                ]
                .spacing(5)
                .width(Length::Fill),
            ]
            .spacing(14)
            .align_y(Alignment::Center),
            channel_row(
                "R",
                self.state.color.r,
                iced_color(Rgb::new(0xFF, 0x5A, 0x36)),
                Message::RedChanged
            ),
            channel_row(
                "G",
                self.state.color.g,
                iced_color(Rgb::new(0x39, 0xD9, 0x8A)),
                Message::GreenChanged
            ),
            channel_row(
                "B",
                self.state.color.b,
                iced_color(Rgb::new(0x6E, 0xA8, 0xFE)),
                Message::BlueChanged
            ),
            row![
                text("Atalhos").size(11).color(C_MUTED),
                preset_button(Rgb::WHITE),
                preset_button(Rgb::new(0xFF, 0x57, 0x33)),
                preset_button(Rgb::new(0x00, 0xFF, 0x88)),
                preset_button(Rgb::new(0x3B, 0x82, 0xF6)),
                preset_button(Rgb::new(0xA8, 0x55, 0xF7)),
                preset_button(Rgb::new(0xFF, 0x2D, 0x55)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(14)
        .width(Length::Fill);

        let color_card = card(
            column![
                row![
                    text("Cor").size(16).font(bold()),
                    Space::new().width(Length::Fill),
                    text(self.state.color.to_hex())
                        .size(12)
                        .font(Font::MONOSPACE)
                        .color(C_MUTED),
                ]
                .align_y(Alignment::Center),
                row![picker_column, controls]
                    .spacing(20)
                    .align_y(Alignment::Start),
            ]
            .spacing(14),
        );

        let brightness_card = card(
            row![
                column![
                    text("Brilho").size(16).font(bold()),
                    text(format!("{}%", brightness_percent(self.state.brightness)))
                        .size(28)
                        .font(bold())
                        .color(C_TEXT),
                ]
                .spacing(2)
                .width(Length::Fixed(90.0)),
                column![
                    slider(
                        0..=MAX_BRIGHTNESS,
                        self.state.brightness,
                        Message::BrightnessChanged
                    )
                    .step(1u8)
                    .width(Length::Fill)
                    .style(|_, status| slider_style(C_ACCENT, status)),
                    row![
                        text("0%").size(10).color(C_MUTED),
                        Space::new().width(Length::Fill),
                        text("25%").size(10).color(C_MUTED),
                        Space::new().width(Length::Fill),
                        text("50%").size(10).color(C_MUTED),
                        Space::new().width(Length::Fill),
                        text("75%").size(10).color(C_MUTED),
                        Space::new().width(Length::Fill),
                        text("100%").size(10).color(C_MUTED),
                    ],
                ]
                .spacing(5)
                .width(Length::Fill),
                column![
                    text("Efeito atual").size(10).color(C_MUTED),
                    text(self.state.effect.label()).size(12).color(C_TEXT),
                ]
                .spacing(3)
                .width(Length::Fixed(150.0)),
            ]
            .spacing(20)
            .align_y(Alignment::Center),
        );

        let effect_buttons = effect_grid(self.state.effect);
        let effects_card = card(
            column![
                row![
                    text("Efeitos").size(16).font(bold()),
                    Space::new().width(Length::Fill),
                    badge(self.state.effect.label(), C_ACCENT),
                ]
                .align_y(Alignment::Center),
                effect_buttons,
            ]
            .spacing(14),
        );

        let (status_text, status_color) = if self.config.device().is_none() {
            ("Configure o teclado na página Conexão", C_WARNING)
        } else if self.auto_apply_paused {
            ("Falha na última aplicação; veja o aviso", C_ERROR)
        } else if self.current_candidate().is_none() {
            ("Teclado ausente; conecte para sincronizar", C_WARNING)
        } else if self.picker_active {
            ("Solte o mouse para aplicar a cor", C_INFO)
        } else if self.dirty {
            ("Aplicando automaticamente (máx. ~8/s)...", C_WARNING)
        } else if self.pending_save {
            ("Sincronizado · salvando configuração...", C_MUTED)
        } else {
            ("Sincronizado automaticamente · máx. ~8 envios/s", C_MUTED)
        };

        let mut actions = row![
            button(text("Restaurar padrão").size(13))
                .padding([10, 14])
                .style(secondary_button)
                .on_press(Message::ResetDefaults),
            Space::new().width(Length::Fill),
            text(status_text).size(11).color(status_color),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        if self.auto_apply_paused && self.config.device().is_some() {
            actions = actions.push(
                button(text("Tentar novamente").size(12))
                    .padding([9, 14])
                    .style(secondary_button)
                    .on_press(Message::Apply),
            );
        }

        content
            .push(color_card)
            .push(brightness_card)
            .push(effects_card)
            .push(actions)
            .into()
    }

    fn connection_page(&self) -> Element<'_, Message> {
        let header = self.page_header(
            "Conexão",
            "Selecione a interface de iluminação do teclado ou informe os IDs manualmente.",
        );

        let mut content = column![header].spacing(18);
        if let Some(notice) = self.notice_view() {
            content = content.push(notice);
        }

        content = content.push(self.current_device_card());
        content = content.push(self.detected_devices_card());
        content = content.push(self.manual_card());
        content = content.push(udev_card());
        content.into()
    }

    fn current_device_card(&self) -> Element<'_, Message> {
        let body: Element<'_, Message> = match self.config.device() {
            Some(saved) => match self.current_candidate() {
                Some(candidate) => column![
                    row![
                        text(candidate.display_name()).size(18).font(bold()),
                        badge("Em uso", C_SUCCESS),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    text(candidate.ids()).size(12).color(C_MUTED),
                    row![
                        button(text("Testar luz roxa").size(12))
                            .padding([8, 12])
                            .style(secondary_button)
                            .on_press(Message::TestDevice(candidate.path.clone())),
                        button(text("Esquecer dispositivo").size(12))
                            .padding([8, 12])
                            .style(danger_button)
                            .on_press(Message::ForgetDevice),
                    ]
                    .spacing(8),
                ]
                .spacing(8)
                .into(),
                None => column![
                    text("Dispositivo configurado, mas não encontrado")
                        .size(17)
                        .font(bold())
                        .color(C_WARNING),
                    text(format!(
                        "{:04X}:{:04X} · interface {}",
                        saved.vendor_id,
                        saved.product_id,
                        saved
                            .interface_number
                            .map(|iface| iface.to_string())
                            .unwrap_or_else(|| "auto".to_string())
                    ))
                    .size(12)
                    .color(C_MUTED),
                    text("Conecte o teclado e clique em Detectar novamente, ou escolha outro dispositivo abaixo.")
                        .size(12)
                        .color(C_MUTED),
                    row![
                        button(text("Detectar novamente").size(12))
                            .padding([8, 12])
                            .style(primary_button)
                            .on_press(Message::Rescan),
                        button(text("Esquecer dispositivo").size(12))
                            .padding([8, 12])
                            .style(danger_button)
                            .on_press(Message::ForgetDevice),
                    ]
                    .spacing(8),
                ]
                .spacing(8)
                .into(),
            },
            None => column![
                text("Nenhum teclado configurado").size(17).font(bold()),
                text("Escolha um dispositivo detectado ou preencha os IDs manualmente.")
                    .size(12)
                    .color(C_MUTED),
            ]
            .spacing(6)
            .into(),
        };

        card(
            column![
                text("Dispositivo atual")
                    .size(14)
                    .font(bold())
                    .color(C_MUTED),
                body,
            ]
            .spacing(12),
        )
        .into()
    }

    fn detected_devices_card(&self) -> Element<'_, Message> {
        let visible: Vec<_> = self
            .candidates
            .iter()
            .filter(|candidate| self.show_all_devices || candidate.vendor_specific)
            .collect();

        let mut list = column![].spacing(8);
        if visible.is_empty() {
            list = list.push(
                text("Nenhum dispositivo HID visível. Clique em Detectar novamente.")
                    .size(12)
                    .color(C_MUTED),
            );
        } else {
            for candidate in visible {
                list = list.push(self.candidate_row(candidate));
            }
        }

        card(
            column![
                row![
                    text("Dispositivos detectados").size(16).font(bold()),
                    Space::new().width(Length::Fill),
                    text(format!("{} encontrado(s)", self.candidates.len()))
                        .size(11)
                        .color(C_MUTED),
                    button(text("Detectar novamente").size(12))
                        .padding([7, 11])
                        .style(secondary_button)
                        .on_press(Message::Rescan),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                row![
                    iced::widget::toggler(self.show_all_devices)
                        .label("Mostrar todos os dispositivos HID")
                        .text_size(12)
                        .on_toggle(Message::ShowAllToggled),
                    Space::new().width(Length::Fill),
                ],
                list,
            ]
            .spacing(12),
        )
        .into()
    }

    fn candidate_row(&self, candidate: &DeviceCandidate) -> Element<'_, Message> {
        let in_use = self.is_in_use(candidate);
        let vendor_badge = if candidate.vendor_specific {
            badge("vendor-specific", C_ACCENT)
        } else {
            badge("outro HID", C_MUTED)
        };

        let mut actions = row![
            button(text("Testar").size(12))
                .padding([7, 11])
                .style(secondary_button)
                .on_press(Message::TestDevice(candidate.path.clone())),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        if in_use {
            actions = actions.push(
                button(text("Em uso").size(12))
                    .padding([7, 11])
                    .style(secondary_button),
            );
        } else {
            actions = actions.push(
                button(text("Usar este dispositivo").size(12))
                    .padding([7, 11])
                    .style(primary_button)
                    .on_press(Message::UseDevice(candidate.path.clone())),
            );
        }

        container(
            row![
                column![
                    row![
                        text(candidate.display_name()).size(14).font(bold()),
                        vendor_badge,
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    text(candidate.ids()).size(11).color(C_MUTED),
                ]
                .spacing(4)
                .width(Length::Fill),
                actions,
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .padding(12)
        .width(Length::Fill)
        .style(move |_| candidate_row_style(in_use))
        .into()
    }

    fn manual_card(&self) -> Element<'_, Message> {
        card(
            column![
                text("Configuração manual").size(16).font(bold()),
                text("Use quando o dispositivo não aparecer corretamente na lista. Campos vazios de interface e usage page são tratados como automáticos.")
                    .size(11)
                    .color(C_MUTED),
                row![
                    field("Vendor ID", "0C45", &self.manual_vid, Message::ManualVidChanged),
                    field("Product ID", "8501", &self.manual_pid, Message::ManualPidChanged),
                    field("Interface", "auto", &self.manual_iface, Message::ManualIfaceChanged),
                    field("Usage page", "opcional", &self.manual_usage, Message::ManualUsageChanged),
                    button(text("Conectar").size(13).font(bold()))
                        .padding([10, 16])
                        .style(primary_button)
                        .on_press(Message::ManualConnect),
                ]
                .spacing(10)
                .align_y(Alignment::End),
            ]
            .spacing(12),
        )
        .into()
    }

    fn about_page(&self) -> Element<'_, Message> {
        let header = self.page_header(
            "Sobre",
            "Alternativa livre ao software oficial do Husky Sled, agora em Rust.",
        );

        let config_path = config::config_file()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "indisponível".to_string());

        let mut content = column![header].spacing(18);
        if let Some(notice) = self.notice_view() {
            content = content.push(notice);
        }

        let about = card(
            column![
                row![
                    image(self.logo.clone()).width(64).height(64),
                    column![
                        text("HuskyNext").size(24).font(bold()),
                        text(format!("Versão {VERSION} · protocolo não oficial"))
                            .size(12)
                            .color(C_MUTED),
                    ]
                    .spacing(3),
                ]
                .spacing(16)
                .align_y(Alignment::Center),
                text("Este projeto não tem vínculo com a Husky. O protocolo foi obtido por engenharia reversa e pode variar entre modelos ou revisões de firmware. Use por sua conta e risco.")
                    .size(13)
                    .color(C_MUTED),
                rule(),
                text("Protocolo").size(16).font(bold()),
                key_value("Report ID", "0x04 · output report"),
                key_value("Tamanho", "64 bytes (1 byte de ID + 63 de payload)"),
                key_value("Modo", "offset 0x09 · 19 efeitos (0x00–0x12)"),
                key_value("Brilho", "offset 0x0A · 5 níveis (0x00–0x04)"),
                key_value("Cor", "offsets 0x0E, 0x0F e 0x10 · R, G, B"),
                key_value("Configuração", config_path),
            ]
            .spacing(12),
        );

        let safety = card(
            column![
                text("Segurança e robustez").size(16).font(bold()),
                bullet("A configuração é gravada de forma atômica, com permissões restritas ao usuário."),
                bullet("Valores inválidos do arquivo são saneados antes de qualquer envio."),
                bullet("O dispositivo é redescoberto a cada envio; IDs ou interface antigos não impedem a reconexão."),
                bullet("O teste de identificação envia roxo (#8835E9) estático a 100%, em vez de prometer uma cor e enviar outra."),
                bullet("A regra udev recomendada usa TAG+=\"uaccess\" em vez de liberar o dispositivo para todos com MODE=\"0666\"."),
            ]
            .spacing(7),
        );

        content.push(about).push(safety).into()
    }

    fn page_header(&self, title: &'static str, subtitle: &'static str) -> Element<'_, Message> {
        column![
            text(title).size(30).font(bold()),
            text(subtitle).size(13).color(C_MUTED),
        ]
        .spacing(4)
        .into()
    }

    fn notice_view(&self) -> Option<Element<'_, Message>> {
        let notice = self.notice.as_ref()?;
        let color = notice.kind.color();

        let mut body = column![
            row![
                text(notice.title.clone())
                    .size(13)
                    .font(bold())
                    .color(color),
                Space::new().width(Length::Fill),
                button(text("Fechar").size(10))
                    .padding([3, 8])
                    .style(ghost_button)
                    .on_press(Message::DismissNotice),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(4);

        if let Some(detail) = &notice.detail {
            body = body.push(
                text(detail.clone())
                    .size(11)
                    .color(C_TEXT)
                    .width(Length::Fill),
            );
        }

        Some(
            container(body)
                .padding(12)
                .width(Length::Fill)
                .style(move |_| notice_style(color))
                .into(),
        )
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

pub fn run() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("HuskyNext — Husky Sled")
        .theme(App::theme)
        .subscription(App::subscription)
        .antialiasing(true)
        .window(window_settings())
        .run()
}

fn window_settings() -> window::Settings {
    let mut settings = window::Settings {
        size: Size::new(1060.0, 760.0),
        min_size: Some(Size::new(900.0, 640.0)),
        ..window::Settings::default()
    };

    #[cfg(target_os = "linux")]
    {
        settings.platform_specific.application_id = "huskynext".to_string();
    }

    settings
}

#[derive(Debug, Clone, Copy)]
struct ColorPicker {
    hsv: Hsv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PickerDrag {
    Square,
    Hue,
}

#[derive(Debug, Default)]
struct ColorPickerState {
    drag: Option<PickerDrag>,
}

impl ColorPicker {
    fn color_at(&self, drag: PickerDrag, point: Point) -> Hsv {
        match drag {
            PickerDrag::Square => Hsv {
                h: self.hsv.h,
                s: (point.x / PICKER_SIZE).clamp(0.0, 1.0),
                v: (1.0 - point.y / PICKER_SIZE).clamp(0.0, 1.0),
            },
            PickerDrag::Hue => Hsv {
                h: ((point.x / PICKER_SIZE).clamp(0.0, 1.0) * 360.0).min(359.9),
                s: self.hsv.s,
                v: self.hsv.v,
            },
        }
    }
}

impl canvas::Program<Message> for ColorPicker {
    type State = ColorPickerState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let local = cursor.position_from(Point::new(bounds.x, bounds.y));

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let point = local?;
                let drag = if picker_square().contains(point) {
                    PickerDrag::Square
                } else if picker_hue().contains(point) {
                    PickerDrag::Hue
                } else {
                    return None;
                };

                state.drag = Some(drag);
                Some(
                    canvas::Action::publish(Message::PickerColor(self.color_at(drag, point)))
                        .and_capture(),
                )
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let drag = state.drag?;
                let point = local?;

                Some(
                    canvas::Action::publish(Message::PickerColor(self.color_at(drag, point)))
                        .and_capture(),
                )
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => state
                .drag
                .take()
                .map(|_| canvas::Action::publish(Message::PickerReleased).and_capture()),
            canvas::Event::Mouse(mouse::Event::CursorLeft) => state
                .drag
                .take()
                .map(|_| canvas::Action::publish(Message::PickerReleased).and_capture()),
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let square = picker_square();
        let hue = picker_hue();

        let hue_color = iced_color(
            Hsv {
                h: self.hsv.h,
                s: 1.0,
                v: 1.0,
            }
            .to_rgb(),
        );

        frame.fill_rectangle(square.position(), square.size(), hue_color);

        let white_to_clear = canvas::gradient::Linear::new(
            Point::new(square.x, square.y),
            Point::new(square.x + square.width, square.y),
        )
        .add_stop(0.0, Color::WHITE)
        .add_stop(1.0, Color::from_rgba(1.0, 1.0, 1.0, 0.0));
        frame.fill_rectangle(square.position(), square.size(), white_to_clear);

        let clear_to_black = canvas::gradient::Linear::new(
            Point::new(square.x, square.y),
            Point::new(square.x, square.y + square.height),
        )
        .add_stop(0.0, Color::from_rgba(0.0, 0.0, 0.0, 0.0))
        .add_stop(1.0, Color::BLACK);
        frame.fill_rectangle(square.position(), square.size(), clear_to_black);

        frame.stroke_rectangle(
            square.position(),
            square.size(),
            canvas::Stroke::default()
                .with_color(C_BORDER_LIGHT)
                .with_width(1.0),
        );

        let mut hue_gradient = canvas::gradient::Linear::new(
            Point::new(hue.x, hue.y + hue.height / 2.0),
            Point::new(hue.x + hue.width, hue.y + hue.height / 2.0),
        );
        for index in 0..=6 {
            hue_gradient = hue_gradient.add_stop(
                index as f32 / 6.0,
                iced_color(
                    Hsv {
                        h: index as f32 * 60.0,
                        s: 1.0,
                        v: 1.0,
                    }
                    .to_rgb(),
                ),
            );
        }
        frame.fill_rectangle(hue.position(), hue.size(), hue_gradient);
        frame.stroke_rectangle(
            hue.position(),
            hue.size(),
            canvas::Stroke::default()
                .with_color(C_BORDER_LIGHT)
                .with_width(1.0),
        );

        let marker = Point::new(
            square.x + self.hsv.s * square.width,
            square.y + (1.0 - self.hsv.v) * square.height,
        );
        frame.stroke(
            &canvas::Path::circle(marker, 7.0),
            canvas::Stroke::default()
                .with_color(Color::from_rgba(0.0, 0.0, 0.0, 0.75))
                .with_width(3.0),
        );
        frame.stroke(
            &canvas::Path::circle(marker, 6.0),
            canvas::Stroke::default()
                .with_color(Color::WHITE)
                .with_width(2.0),
        );

        let hue_x = (self.hsv.h / 360.0).clamp(0.0, 1.0);
        let hue_marker = Point::new(hue.x + hue_x * hue.width, hue.y + hue.height / 2.0);
        frame.fill(&canvas::Path::circle(hue_marker, 7.0), Color::WHITE);
        frame.stroke(
            &canvas::Path::circle(hue_marker, 7.0),
            canvas::Stroke::default()
                .with_color(Color::from_rgba(0.0, 0.0, 0.0, 0.75))
                .with_width(1.5),
        );

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.drag.is_some() {
            return mouse::Interaction::Grabbing;
        }

        let local = cursor.position_from(Point::new(bounds.x, bounds.y));
        if local
            .is_some_and(|point| picker_square().contains(point) || picker_hue().contains(point))
        {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::None
        }
    }
}

fn picker_square() -> Rectangle {
    Rectangle::new(Point::new(0.0, 0.0), Size::new(PICKER_SIZE, PICKER_SIZE))
}

fn picker_hue() -> Rectangle {
    Rectangle::new(
        Point::new(0.0, PICKER_SIZE + HUE_GAP),
        Size::new(PICKER_SIZE, HUE_HEIGHT),
    )
}

fn effect_grid(selected: Effect) -> Element<'static, Message> {
    let mut grid = column![].spacing(8);

    for chunk in Effect::ALL.chunks(4) {
        let mut grid_row = row![].spacing(8);
        for effect in chunk.iter().copied() {
            grid_row = grid_row.push(effect_button(effect, selected));
        }

        for _ in chunk.len()..4 {
            grid_row = grid_row.push(Space::new().width(Length::Fill));
        }

        grid = grid.push(grid_row);
    }

    grid.into()
}

fn effect_button(effect: Effect, selected: Effect) -> Element<'static, Message> {
    let is_selected = effect == selected;

    button(
        text(effect.label())
            .size(12)
            .align_x(Alignment::Center)
            .width(Length::Fill),
    )
    .padding([9, 8])
    .width(Length::Fill)
    .height(Length::Fixed(40.0))
    .style(move |_, status| effect_button_style(is_selected, status))
    .on_press(Message::EffectSelected(effect))
    .into()
}

fn channel_row<'a>(
    label: &'static str,
    value: u8,
    color: Color,
    on_change: impl Fn(u8) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        text(label)
            .size(12)
            .font(bold())
            .width(Length::Fixed(16.0))
            .color(color),
        slider(0..=255u8, value, on_change)
            .width(Length::Fill)
            .style(move |_, status| slider_style(color, status)),
        text(format!("{value:3}"))
            .size(11)
            .font(Font::MONOSPACE)
            .width(Length::Fixed(32.0))
            .color(C_MUTED),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

fn preset_button(rgb: Rgb) -> Element<'static, Message> {
    let color = iced_color(rgb);

    button("")
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .style(move |_, _| preset_style(color))
        .on_press(Message::PresetColor(rgb))
        .into()
}

fn field<'a>(
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    column![
        text(label).size(10).color(C_MUTED),
        text_input(placeholder, value)
            .on_input(on_input)
            .on_submit(Message::ManualConnect)
            .padding(9)
            .size(12)
            .style(input_style(false)),
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}

fn nav_button(label: &'static str, page: Page, current: Page) -> Element<'static, Message> {
    let active = page == current;

    button(
        text(label)
            .size(13)
            .font(if active { bold() } else { Font::DEFAULT }),
    )
    .padding([10, 12])
    .width(Length::Fill)
    .style(move |_, status| nav_style(active, status))
    .on_press(Message::Navigate(page))
    .into()
}

fn card<'a>(content: impl Into<Element<'a, Message>>) -> Container<'a, Message> {
    container(content)
        .padding(20)
        .width(Length::Fill)
        .style(card_style)
}

fn badge(label: impl Into<String>, color: Color) -> Element<'static, Message> {
    container(text(label.into()).size(10).color(color))
        .padding([3, 8])
        .style(move |_| badge_style(color))
        .into()
}

fn dot(color: Color) -> Element<'static, Message> {
    container("")
        .width(Length::Fixed(8.0))
        .height(Length::Fixed(8.0))
        .style(move |_| ContainerStyle {
            background: Some(Background::Color(color)),
            border: Border {
                radius: 4.0.into(),
                ..Border::default()
            },
            ..ContainerStyle::default()
        })
        .into()
}

fn rule<'a>() -> Element<'a, Message> {
    container("")
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(|_| ContainerStyle {
            background: Some(Background::Color(C_BORDER)),
            ..ContainerStyle::default()
        })
        .into()
}

fn key_value(key: &'static str, value: impl Into<String>) -> Element<'static, Message> {
    row![
        text(key)
            .size(11)
            .color(C_MUTED)
            .width(Length::Fixed(110.0)),
        text(value.into())
            .size(11)
            .font(Font::MONOSPACE)
            .color(C_TEXT)
            .width(Length::Fill),
    ]
    .spacing(12)
    .into()
}

fn bullet<'a>(text_value: &'a str) -> Element<'a, Message> {
    row![
        text("•").size(13).color(C_ACCENT),
        text(text_value).size(12).color(C_MUTED).width(Length::Fill),
    ]
    .spacing(8)
    .into()
}

fn udev_card<'a>() -> Element<'a, Message> {
    card(
        column![
            text("Permissão no Linux").size(16).font(bold()),
            text("Se aparecer “sem permissão”, crie /etc/udev/rules.d/99-huskynext.rules com o conteúdo abaixo. Substitua os IDs pelos do seu teclado, se necessário.")
                .size(12)
                .color(C_MUTED),
            container(
                text("SUBSYSTEM==\"usb\", ATTRS{idVendor}==\"0c45\", ATTRS{idProduct}==\"8501\", TAG+=\"uaccess\"\nSUBSYSTEM==\"hidraw\", ATTRS{idVendor}==\"0c45\", ATTRS{idProduct}==\"8501\", TAG+=\"uaccess\"")
                    .size(11)
                    .font(Font::MONOSPACE)
                    .color(C_TEXT)
            )
            .padding(12)
            .width(Length::Fill)
            .style(|_| ContainerStyle {
                background: Some(Background::Color(C_INPUT)),
                border: Border {
                    color: C_BORDER,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..ContainerStyle::default()
            }),
            text("Depois: sudo udevadm control --reload-rules && sudo udevadm trigger. Reconecte o teclado. TAG+=\"uaccess\" concede acesso apenas à sessão ativa, sem abrir o dispositivo para todos os usuários.")
                .size(11)
                .color(C_MUTED),
        ]
        .spacing(10),
    )
    .into()
}

fn device_error_notice(title: &str, error: &DeviceError) -> Notice {
    let mut notice = Notice::error(title.to_string()).with_detail(error.to_string());
    if let Some(hint) = error.user_hint() {
        notice = notice.with_detail(format!("{error}\n\n{hint}"));
    }
    notice
}

fn parse_hex_u16(value: &str) -> Option<u16> {
    let value = value.trim();
    let value = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    u16::from_str_radix(value, 16).ok()
}

fn iced_color(rgb: Rgb) -> Color {
    Color::from_rgb8(rgb.r, rgb.g, rgb.b)
}

fn bold() -> Font {
    Font {
        weight: iced::font::Weight::Bold,
        ..Font::DEFAULT
    }
}

fn sidebar_style(_theme: &Theme) -> ContainerStyle {
    ContainerStyle {
        background: Some(Background::Color(C_SIDEBAR)),
        border: Border {
            color: C_BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..ContainerStyle::default()
    }
}

fn sidebar_status_style(_theme: &Theme) -> ContainerStyle {
    ContainerStyle {
        background: Some(Background::Color(C_SURFACE)),
        border: Border {
            color: C_BORDER,
            width: 1.0,
            radius: 10.0.into(),
        },
        ..ContainerStyle::default()
    }
}

fn card_style(_theme: &Theme) -> ContainerStyle {
    ContainerStyle {
        background: Some(Background::Color(C_SURFACE)),
        border: Border {
            color: C_BORDER,
            width: 1.0,
            radius: 14.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.22),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        ..ContainerStyle::default()
    }
}

fn preview_style(color: Color) -> impl Fn(&Theme) -> ContainerStyle {
    move |_| ContainerStyle {
        background: Some(Background::Color(color)),
        border: Border {
            color: Color::from_rgba8(255, 255, 255, 0.22),
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.35),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..ContainerStyle::default()
    }
}

fn nav_style(active: bool, status: ButtonStatus) -> ButtonStyle {
    let (background, text_color) = if active {
        (Some(C_ACCENT.into()), Color::WHITE)
    } else {
        match status {
            ButtonStatus::Hovered => (Some(C_SURFACE_ALT.into()), C_TEXT),
            ButtonStatus::Pressed => (Some(C_SURFACE.into()), C_TEXT),
            _ => (None, C_MUTED),
        }
    };

    ButtonStyle {
        background,
        text_color,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 9.0.into(),
        },
        ..ButtonStyle::default()
    }
}

fn effect_button_style(selected: bool, status: ButtonStatus) -> ButtonStyle {
    let (background, text_color, border_color) = if selected {
        match status {
            ButtonStatus::Hovered | ButtonStatus::Pressed => {
                (C_ACCENT_HOVER, Color::WHITE, C_ACCENT_HOVER)
            }
            _ => (C_ACCENT, Color::WHITE, C_ACCENT),
        }
    } else {
        let background = match status {
            ButtonStatus::Hovered => C_SURFACE_ALT,
            ButtonStatus::Pressed => C_INPUT,
            ButtonStatus::Disabled => C_INPUT,
            _ => C_SURFACE_ALT,
        };
        (background, C_TEXT, C_BORDER)
    };

    ButtonStyle {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 9.0.into(),
        },
        ..ButtonStyle::default()
    }
}

fn primary_button(_theme: &Theme, status: ButtonStatus) -> ButtonStyle {
    let background = match status {
        ButtonStatus::Hovered => C_ACCENT_HOVER,
        ButtonStatus::Pressed => C_ACCENT_PRESSED,
        ButtonStatus::Disabled => C_TRACK,
        ButtonStatus::Active => C_ACCENT,
    };
    let text_color = if status == ButtonStatus::Disabled {
        C_MUTED
    } else {
        Color::WHITE
    };

    ButtonStyle {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 10.0.into(),
        },
        ..ButtonStyle::default()
    }
}

fn secondary_button(_theme: &Theme, status: ButtonStatus) -> ButtonStyle {
    let background = match status {
        ButtonStatus::Hovered => Some(C_SURFACE_ALT.into()),
        ButtonStatus::Pressed => Some(C_INPUT.into()),
        _ => None,
    };

    ButtonStyle {
        background,
        text_color: C_TEXT,
        border: Border {
            color: C_BORDER,
            width: 1.0,
            radius: 10.0.into(),
        },
        ..ButtonStyle::default()
    }
}

fn danger_button(_theme: &Theme, status: ButtonStatus) -> ButtonStyle {
    let background = match status {
        ButtonStatus::Hovered => Some(Color::from_rgba8(255, 107, 107, 0.14).into()),
        ButtonStatus::Pressed => Some(Color::from_rgba8(255, 107, 107, 0.22).into()),
        _ => None,
    };

    ButtonStyle {
        background,
        text_color: C_ERROR,
        border: Border {
            color: Color::from_rgba8(255, 107, 107, 0.45),
            width: 1.0,
            radius: 10.0.into(),
        },
        ..ButtonStyle::default()
    }
}

fn ghost_button(_theme: &Theme, status: ButtonStatus) -> ButtonStyle {
    ButtonStyle {
        background: match status {
            ButtonStatus::Hovered => Some(C_SURFACE_ALT.into()),
            ButtonStatus::Pressed => Some(C_INPUT.into()),
            _ => None,
        },
        text_color: C_MUTED,
        border: Border::default(),
        ..ButtonStyle::default()
    }
}

fn preset_style(color: Color) -> ButtonStyle {
    ButtonStyle {
        background: Some(Background::Color(color)),
        text_color: Color::TRANSPARENT,
        border: Border {
            color: Color::from_rgba8(255, 255, 255, 0.28),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..ButtonStyle::default()
    }
}

fn input_style(error: bool) -> impl Fn(&Theme, InputStatus) -> InputStyle {
    move |_, status| {
        let border_color = if error {
            C_ERROR
        } else {
            match status {
                InputStatus::Focused { .. } => C_ACCENT,
                InputStatus::Hovered => C_BORDER_LIGHT,
                _ => C_BORDER,
            }
        };

        InputStyle {
            background: C_INPUT.into(),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 9.0.into(),
            },
            icon: C_MUTED,
            placeholder: C_MUTED,
            value: C_TEXT,
            selection: Color::from_rgba8(255, 90, 54, 0.35),
        }
    }
}

fn slider_style(color: Color, _status: SliderStatus) -> SliderStyle {
    SliderStyle {
        rail: Rail {
            backgrounds: (color.into(), C_TRACK.into()),
            width: 5.0,
            border: Border {
                radius: 3.0.into(),
                ..Border::default()
            },
        },
        handle: SliderHandle {
            shape: HandleShape::Circle { radius: 8.0 },
            background: color.into(),
            border_width: 2.0,
            border_color: C_SURFACE,
        },
    }
}

fn notice_style(color: Color) -> ContainerStyle {
    ContainerStyle {
        background: Some(
            Color::from_rgba8(
                (color.r * 255.0) as u8,
                (color.g * 255.0) as u8,
                (color.b * 255.0) as u8,
                0.10,
            )
            .into(),
        ),
        border: Border {
            color: Color::from_rgba8(
                (color.r * 255.0) as u8,
                (color.g * 255.0) as u8,
                (color.b * 255.0) as u8,
                0.45,
            ),
            width: 1.0,
            radius: 10.0.into(),
        },
        ..ContainerStyle::default()
    }
}

fn badge_style(color: Color) -> ContainerStyle {
    ContainerStyle {
        background: Some(
            Color::from_rgba8(
                (color.r * 255.0) as u8,
                (color.g * 255.0) as u8,
                (color.b * 255.0) as u8,
                0.14,
            )
            .into(),
        ),
        border: Border {
            color: Color::from_rgba8(
                (color.r * 255.0) as u8,
                (color.g * 255.0) as u8,
                (color.b * 255.0) as u8,
                0.38,
            ),
            width: 1.0,
            radius: 20.0.into(),
        },
        ..ContainerStyle::default()
    }
}

fn candidate_row_style(in_use: bool) -> ContainerStyle {
    ContainerStyle {
        background: Some(Background::Color(if in_use {
            C_SURFACE_ALT
        } else {
            C_INPUT
        })),
        border: Border {
            color: if in_use { C_ACCENT } else { C_BORDER },
            width: 1.0,
            radius: 10.0.into(),
        },
        ..ContainerStyle::default()
    }
}
