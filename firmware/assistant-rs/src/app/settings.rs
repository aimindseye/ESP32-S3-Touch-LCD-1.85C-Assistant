use crate::app::wifi::WifiProvisioningStep;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsPanel {
    Network,
    Weather,
    Time,
    Display,
    Sound,
    Storage,
    Device,
    Diagnostics,
}

impl SettingsPanel {
    pub const fn row_label(self) -> &'static str {
        match self {
            Self::Network => "NETWORK",
            Self::Weather => "WEATHER",
            Self::Time => "TIME",
            Self::Display => "DISPLAY",
            Self::Sound => "SOUND",
            Self::Storage => "STORAGE",
            Self::Device => "DEVICE",
            Self::Diagnostics => "DIAG",
        }
    }

    pub const fn detail_title(self) -> &'static str {
        self.row_label()
    }

    pub const fn log_label(self) -> &'static str {
        match self {
            Self::Network => "network",
            Self::Weather => "weather",
            Self::Time => "time",
            Self::Display => "display",
            Self::Sound => "sound",
            Self::Storage => "storage",
            Self::Device => "device",
            Self::Diagnostics => "diagnostics",
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Network => Self::Weather,
            Self::Weather => Self::Time,
            Self::Time => Self::Display,
            Self::Display => Self::Sound,
            Self::Sound => Self::Storage,
            Self::Storage => Self::Device,
            Self::Device => Self::Diagnostics,
            Self::Diagnostics => Self::Network,
        }
    }

    pub const fn previous(self) -> Self {
        match self {
            Self::Network => Self::Diagnostics,
            Self::Weather => Self::Network,
            Self::Time => Self::Weather,
            Self::Display => Self::Time,
            Self::Sound => Self::Display,
            Self::Storage => Self::Sound,
            Self::Device => Self::Storage,
            Self::Diagnostics => Self::Device,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SettingsState {
    pub detail_open: bool,
    pub selected: SettingsPanel,
    pub overview_page: u8,
    pub toggle_count: u32,
    pub touch_trace_enabled: bool,
    pub quiet_render_enabled: bool,
    pub brightness_percent: u8,
    pub volume_percent: u8,
    pub wifi_mock_connected: bool,
    pub wifi_scan_count: u32,
    pub wifi_provisioning: WifiProvisioningStep,
    pub wifi_provision_attempts: u32,
    pub wifi_connect_requested: bool,
    pub wifi_last_error: &'static str,
    pub about_page: u8,
    pub software_sleep_requested: bool,
}

impl SettingsState {
    pub const fn new() -> Self {
        Self {
            detail_open: false,
            selected: SettingsPanel::Network,
            overview_page: 0,
            toggle_count: 0,
            touch_trace_enabled: true,
            quiet_render_enabled: true,
            brightness_percent: 70,
            volume_percent: 60,
            wifi_mock_connected: false,
            wifi_scan_count: 0,
            wifi_provisioning: WifiProvisioningStep::Idle,
            wifi_provision_attempts: 0,
            wifi_connect_requested: false,
            wifi_last_error: "NONE",
            about_page: 0,
            software_sleep_requested: false,
        }
    }

    pub fn panel_for_touch_y(&self, y: u16) -> SettingsPanel {
        let row = match y {
            82..=135 => 0,
            136..=185 => 1,
            186..=235 => 2,
            236..=300 => 3,
            _ => return self.selected,
        };

        match (self.overview_page % 2, row) {
            (0, 0) => SettingsPanel::Network,
            (0, 1) => SettingsPanel::Weather,
            (0, 2) => SettingsPanel::Time,
            (0, _) => SettingsPanel::Display,
            (_, 0) => SettingsPanel::Sound,
            (_, 1) => SettingsPanel::Storage,
            (_, 2) => SettingsPanel::Device,
            (_, _) => SettingsPanel::Diagnostics,
        }
    }

    pub fn is_overview_page_tap(&self, y: u16) -> bool {
        !self.detail_open && y >= 306
    }

    pub fn next_overview_page(&mut self) {
        self.overview_page = (self.overview_page + 1) % 2;
        self.selected = if self.overview_page == 0 {
            SettingsPanel::Network
        } else {
            SettingsPanel::Sound
        };
    }

    pub fn overview_page_label(&self) -> &'static str {
        if self.overview_page == 0 {
            "1/2"
        } else {
            "2/2"
        }
    }

    pub fn enter_detail_for(&mut self, panel: SettingsPanel) {
        self.selected = panel;
        self.detail_open = true;
    }

    pub fn enter_detail(&mut self) {
        self.detail_open = true;
    }

    pub fn close_detail(&mut self) {
        self.detail_open = false;
    }

    pub fn take_wifi_connect_request(&mut self) -> bool {
        let requested = self.wifi_connect_requested;
        self.wifi_connect_requested = false;
        requested
    }

    pub fn take_software_sleep_request(&mut self) -> bool {
        let requested = self.software_sleep_requested;
        self.software_sleep_requested = false;
        requested
    }

    pub fn set_wifi_stage(&mut self, stage: WifiProvisioningStep, error: &'static str) {
        self.wifi_provisioning = stage;
        self.wifi_last_error = error;
        self.wifi_mock_connected = matches!(stage, WifiProvisioningStep::Connected);
    }

    pub fn next_detail_panel(&mut self) {
        self.selected = self.selected.next();
        self.detail_open = true;
    }

    pub fn previous_detail_panel(&mut self) {
        self.selected = self.selected.previous();
        self.detail_open = true;
    }

    pub fn toggle_current(&mut self) {
        match self.selected {
            SettingsPanel::Network => {
                self.wifi_scan_count = self.wifi_scan_count.saturating_add(1);
                self.wifi_provision_attempts = self.wifi_provision_attempts.saturating_add(1);
                self.wifi_provisioning = WifiProvisioningStep::ImportSd;
                self.wifi_connect_requested = true;
                self.wifi_last_error = "IMPORT";
            }
            SettingsPanel::Display => {
                self.software_sleep_requested = true;
            }
            SettingsPanel::Sound => {
                self.volume_percent = match self.volume_percent {
                    0..=39 => 50,
                    40..=59 => 70,
                    60..=79 => 90,
                    _ => 30,
                };
            }
            SettingsPanel::Device | SettingsPanel::Diagnostics => {
                self.about_page = (self.about_page + 1) % 2;
            }
            SettingsPanel::Weather | SettingsPanel::Time | SettingsPanel::Storage => {}
        }

        self.toggle_count = self.toggle_count.saturating_add(1);
    }

    pub const fn touch_trace_label(&self) -> &'static str {
        if self.touch_trace_enabled {
            "TRACE ON"
        } else {
            "TRACE OFF"
        }
    }

    pub const fn quiet_render_label(&self) -> &'static str {
        if self.quiet_render_enabled {
            "QUIET ON"
        } else {
            "QUIET OFF"
        }
    }

    pub const fn wifi_status_label(&self) -> &'static str {
        self.wifi_provisioning.label()
    }

    pub const fn wifi_provisioning_label(&self) -> &'static str {
        self.wifi_provisioning.label()
    }

    pub const fn wifi_error_label(&self) -> &'static str {
        self.wifi_last_error
    }

    pub fn brightness_label(&self) -> String {
        format!("{}%", self.brightness_percent)
    }

    pub fn volume_label(&self) -> String {
        format!("{}%", self.volume_percent)
    }

    pub fn wifi_scan_label(&self) -> String {
        format!("SCAN {}", self.wifi_scan_count)
    }

    pub fn wifi_provision_attempt_label(&self) -> String {
        format!("TRY {}", self.wifi_provision_attempts)
    }

    pub const fn current_panel_label(&self) -> &'static str {
        self.selected.log_label()
    }

    pub fn current_value_label(&self) -> String {
        match self.selected {
            SettingsPanel::Network => {
                format!(
                    "{} {}",
                    self.wifi_provisioning_label(),
                    self.wifi_provision_attempt_label()
                )
            }
            SettingsPanel::Weather => "CYCLE+FETCH".to_string(),
            SettingsPanel::Time => "SYNC STATUS".to_string(),
            SettingsPanel::Display => "SLEEP NOW".to_string(),
            SettingsPanel::Sound => {
                format!("VOL {}", self.volume_label())
            }
            SettingsPanel::Storage => "CACHE".to_string(),
            SettingsPanel::Device => {
                format!("PAGE {}", self.about_page + 1)
            }
            SettingsPanel::Diagnostics => {
                format!("PAGE {}", self.about_page + 1)
            }
        }
    }
}

pub const SETTINGS_SUBSCREEN_MARKER: &str =
    "v0.1.22 settings details hub: network weather time display sound storage device diagnostics";
