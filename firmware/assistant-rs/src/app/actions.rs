use crate::app::{model::OnboardModel, pages::AssistantPage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    HomeStatus,
    WeatherRefresh,
    MusicTogglePlayPause,
    AssistantListenToggle,
    SettingsEnter,
    SettingsToggle,
}

impl AppAction {
    pub const fn log_marker(self) -> &'static str {
        match self {
            Self::HomeStatus => "action: HomeStatus",
            Self::WeatherRefresh => "action: WeatherRefresh",
            Self::MusicTogglePlayPause => "action: MusicTogglePlayPause",
            Self::AssistantListenToggle => "action: AssistantListenToggle",
            Self::SettingsEnter => "action: SettingsEnter",
            Self::SettingsToggle => "action: SettingsToggle",
        }
    }

    pub const fn status_label(self) -> &'static str {
        match self {
            Self::HomeStatus => "HOME STATUS",
            Self::WeatherRefresh => "WEATHER MOCK",
            Self::MusicTogglePlayPause => "MUSIC TOGGLE",
            Self::AssistantListenToggle => "ASSISTANT",
            Self::SettingsEnter => "SETTINGS OPEN",
            Self::SettingsToggle => "SETTINGS TOGGLE",
        }
    }
}

pub fn handle_select_action(model: &mut OnboardModel) -> AppAction {
    let action = match model.current_page {
        AssistantPage::Home => {
            model.home.toggle_status_detail();
            AppAction::HomeStatus
        }
        AssistantPage::Weather => {
            model.weather.refresh_mock();
            AppAction::WeatherRefresh
        }
        AssistantPage::Music => {
            model.music.toggle_play_pause();
            AppAction::MusicTogglePlayPause
        }
        AssistantPage::Assistant => {
            model.assistant.toggle_listening();
            AppAction::AssistantListenToggle
        }
        AssistantPage::Settings => {
            if model.settings.detail_open {
                model.settings.toggle_current();
                AppAction::SettingsToggle
            } else {
                model.settings.enter_detail();
                AppAction::SettingsEnter
            }
        }
    };

    model.last_action = action.status_label();

    match action {
        AppAction::HomeStatus => {
            println!("screen: HomeStatus {}", model.home.detail_label());
        }
        AppAction::WeatherRefresh => {
            println!(
                "screen: WeatherRefresh sample={} temp={} condition={}",
                model.weather.refresh_attempts,
                model.weather.temperature_label,
                model.weather.condition_label
            );
        }
        AppAction::MusicTogglePlayPause => {
            println!(
                "screen: MusicTogglePlayPause state={} track={}",
                model.music.state_label(),
                model.music.track_label
            );
        }
        AppAction::AssistantListenToggle => {
            println!(
                "screen: AssistantListenToggle state={} toggles={}",
                model.assistant.state_label(),
                model.assistant.toggle_count
            );
        }
        AppAction::SettingsEnter => {
            println!("screen: SettingsEnter detail=open");
        }
        AppAction::SettingsToggle => {
            println!(
                "screen: SettingsToggle {}",
                model.settings.quiet_render_label()
            );
        }
    }

    action
}
