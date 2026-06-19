#[derive(Debug, Clone)]
pub struct AssistantState {
    pub listening: bool,
    pub toggle_count: u32,
}

impl AssistantState {
    pub const fn new() -> Self {
        Self {
            listening: false,
            toggle_count: 0,
        }
    }

    pub fn toggle_listening(&mut self) {
        self.listening = !self.listening;
        self.toggle_count = self.toggle_count.saturating_add(1);
    }

    pub const fn state_label(&self) -> &'static str {
        if self.listening {
            "LISTENING"
        } else {
            "TAP TO TALK"
        }
    }

    pub const fn prompt_label(&self) -> &'static str {
        if self.listening {
            "HOW CAN I HELP"
        } else {
            "LOCAL ASSIST"
        }
    }

    pub const fn title_label(&self) -> &'static str {
        if self.listening {
            "LISTENING"
        } else {
            "AI ASSISTANT"
        }
    }

    pub const fn subtitle_label(&self) -> &'static str {
        if self.listening {
            "AI ASSISTANT"
        } else {
            "LOCAL ASSIST"
        }
    }

    pub const fn card_label(&self) -> &'static str {
        if self.listening {
            "HOW CAN I HELP?"
        } else {
            "TAP TO TALK"
        }
    }

    pub const fn card_aux_label(&self) -> &'static str {
        if self.listening {
            "LOCAL MOCK"
        } else {
            "READY"
        }
    }
}
