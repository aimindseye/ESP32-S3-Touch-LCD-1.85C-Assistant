#[derive(Debug, Clone)]
pub struct SettingsState {
    pub detail_open: bool,
    pub toggle_count: u32,
    pub touch_trace_enabled: bool,
    pub quiet_render_enabled: bool,
}

impl SettingsState {
    pub const fn new() -> Self {
        Self {
            detail_open: false,
            toggle_count: 0,
            touch_trace_enabled: true,
            quiet_render_enabled: true,
        }
    }

    pub fn enter_detail(&mut self) {
        self.detail_open = true;
    }

    pub fn toggle_current(&mut self) {
        self.quiet_render_enabled = !self.quiet_render_enabled;
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
}
