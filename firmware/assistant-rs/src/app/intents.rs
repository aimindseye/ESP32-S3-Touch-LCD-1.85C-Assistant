//! UI intent boundary.
//!
//! Hardware input classifiers emit `UiIntent`. App state dispatch decides how to
//! apply the intent. This keeps touch/button code isolated from provider logic.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiIntent {
    NextPage,
    PreviousPage,
    Select,
    SettingsBackToOverview,
    SettingsNextDetail,
    SettingsPreviousDetail,
    BackHome,
    AssistantHold,
    PowerMenu,
    BootReserved,
}

impl UiIntent {
    pub const fn log_label(self) -> &'static str {
        match self {
            Self::NextPage => "intent: NextPage",
            Self::PreviousPage => "intent: PreviousPage",
            Self::Select => "intent: Select",
            Self::SettingsBackToOverview => "intent: SettingsBackToOverview",
            Self::SettingsNextDetail => "intent: SettingsNextDetail",
            Self::SettingsPreviousDetail => "intent: SettingsPreviousDetail",
            Self::BackHome => "intent: BackHome",
            Self::AssistantHold => "intent: AssistantHold",
            Self::PowerMenu => "intent: PowerMenu",
            Self::BootReserved => "intent: BootReserved",
        }
    }

    pub const fn is_navigation(self) -> bool {
        matches!(self, Self::NextPage | Self::PreviousPage | Self::BackHome)
    }
}

pub const INTENT_BOUNDARY_MARKER: &str =
    "v0.1.16-r4 intent boundary: settings detail header back active";
