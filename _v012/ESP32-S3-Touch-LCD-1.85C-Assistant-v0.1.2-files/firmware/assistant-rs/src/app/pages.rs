#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistantPage {
    Home,
    Weather,
    Music,
    Settings,
}

pub const ALL_PAGES: [AssistantPage; 4] = [
    AssistantPage::Home,
    AssistantPage::Weather,
    AssistantPage::Music,
    AssistantPage::Settings,
];

impl AssistantPage {
    pub const fn title(self) -> &'static str {
        match self {
            Self::Home => "HOME",
            Self::Weather => "WEATHER",
            Self::Music => "MUSIC",
            Self::Settings => "SETTINGS",
        }
    }

    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Home => "HOME",
            Self::Weather => "WEATHER",
            Self::Music => "MUSIC",
            Self::Settings => "SETTINGS",
        }
    }
}
