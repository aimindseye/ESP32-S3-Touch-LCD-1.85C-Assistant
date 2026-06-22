#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistantPage {
    Home,
    Weather,
    Music,
    InternetRadio,
    Assistant,
    Settings,
}

pub const ALL_PAGES: [AssistantPage; 6] = [
    AssistantPage::Home,
    AssistantPage::Weather,
    AssistantPage::Music,
    AssistantPage::InternetRadio,
    AssistantPage::Assistant,
    AssistantPage::Settings,
];

impl AssistantPage {
    pub const fn title(self) -> &'static str {
        match self {
            Self::Home => "HOME",
            Self::Weather => "WEATHER",
            Self::Music => "MUSIC",
            Self::InternetRadio => "RADIO",
            Self::Assistant => "ASSISTANT",
            Self::Settings => "SETTINGS",
        }
    }

    pub const fn index(self) -> usize {
        match self {
            Self::Home => 0,
            Self::Weather => 1,
            Self::Music => 2,
            Self::InternetRadio => 3,
            Self::Assistant => 4,
            Self::Settings => 5,
        }
    }

    pub const fn count() -> usize {
        ALL_PAGES.len()
    }

    pub fn next(self) -> Self {
        let idx = (self.index() + 1) % Self::count();
        ALL_PAGES[idx]
    }

    pub fn previous(self) -> Self {
        let idx = if self.index() == 0 {
            Self::count() - 1
        } else {
            self.index() - 1
        };
        ALL_PAGES[idx]
    }
}

// RAW-R42-VIDEO-PAGE-REMOVED-FROM-ENUM
