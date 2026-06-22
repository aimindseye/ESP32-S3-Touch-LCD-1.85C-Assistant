use crate::app::{
    pages::AssistantPage, providers::LocalProviders, settings::SettingsPanel, state::AppState,
};
use crate::internet_radio;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    HomeStatus,
    WeatherRefresh,
    WeatherLocation,
    WeatherUnits,
    MusicTogglePlayPause,
    InternetRadioToggle,
    AssistantListenToggle,
    SettingsEnter,
    SettingsToggle,
}

impl AppAction {
    pub const fn log_marker(self) -> &'static str {
        match self {
            Self::HomeStatus => "action: HomeStatus",
            Self::WeatherRefresh => "action: WeatherRefresh",
            Self::WeatherLocation => "action: WeatherLocation",
            Self::WeatherUnits => "action: WeatherUnits",
            Self::MusicTogglePlayPause => "action: MusicTogglePlayPause",
            Self::InternetRadioToggle => "action: InternetRadioToggle",
            Self::AssistantListenToggle => "action: AssistantListenToggle",
            Self::SettingsEnter => "action: SettingsEnter",
            Self::SettingsToggle => "action: SettingsToggle",
        }
    }

    pub const fn status_label(self) -> &'static str {
        match self {
            Self::HomeStatus => "HOME REFRESH",
            Self::WeatherRefresh => "WEATHER FETCH",
            Self::WeatherLocation => "WX LOCATION",
            Self::WeatherUnits => "WX UNITS",
            Self::MusicTogglePlayPause => "MUSIC TOGGLE",
            Self::InternetRadioToggle => "RADIO TOGGLE",
            Self::AssistantListenToggle => "ASSISTANT",
            Self::SettingsEnter => "SETTINGS OPEN",
            Self::SettingsToggle => "SETTINGS APPLY",
        }
    }
}

pub fn handle_select_action(model: &mut AppState, providers: &LocalProviders) -> AppAction {
    let action = match model.current_page {
        AssistantPage::Home => {
            providers.toggle_home_status(&mut model.home);
            providers.refresh_weather(&mut model.weather);
            AppAction::HomeStatus
        }
        AssistantPage::Weather => {
            let y = model.last_touch.map(|touch| touch.y).unwrap_or(180);
            // v0.1.21-r2: center/body tap is the location editor action.
            // Changing location auto-fetches immediately in main.rs so the
            // screen does not get stuck on a cached city.
            if y >= 300 {
                providers.toggle_weather_units(&mut model.weather);
                AppAction::WeatherUnits
            } else {
                providers.cycle_weather_location(&mut model.weather);
                AppAction::WeatherLocation
            }
        }
        AssistantPage::Music => {
            providers.toggle_music_play_pause(&mut model.music);
            AppAction::MusicTogglePlayPause
        }
        AssistantPage::InternetRadio => {
            let _ = internet_radio::toggle_play_stop();
            AppAction::InternetRadioToggle
        }
        AssistantPage::Assistant => {
            providers.toggle_assistant_listening(&mut model.assistant);
            AppAction::AssistantListenToggle
        }
        AssistantPage::Settings => {
            if model.settings.detail_open {
                match model.settings.selected {
                    SettingsPanel::Weather => {
                        providers.cycle_weather_location(&mut model.weather);
                        providers.refresh_weather(&mut model.weather);
                    }
                    _ => {
                        providers.toggle_settings_current(&mut model.settings);
                    }
                }
                AppAction::SettingsToggle
            } else {
                let touch_y = model.last_touch.map(|touch| touch.y).unwrap_or(180);
                if model.settings.is_overview_page_tap(touch_y) {
                    providers.next_settings_overview_page(&mut model.settings);
                    AppAction::SettingsToggle
                } else {
                    let selected = model.settings.panel_for_touch_y(touch_y);
                    providers.enter_settings_detail(&mut model.settings, selected);
                    AppAction::SettingsEnter
                }
            }
        }
    };

    model.last_action = action.status_label();

    match action {
        AppAction::HomeStatus => {
            println!(
                "screen: HomeRefresh weather status={} location={}",
                model.weather.status_label(),
                model.weather.location_label()
            );
        }
        AppAction::WeatherRefresh => {
            println!(
                "screen: WeatherRefresh attempt={} temp={} condition={} status={}",
                model.weather.refresh_attempts,
                model.weather.temperature_label(),
                model.weather.condition_label(),
                model.weather.status_label()
            );
        }
        AppAction::WeatherLocation => {
            println!(
                "screen: WeatherLocation location={} units={}",
                model.weather.location_label(),
                model.weather.units.suffix()
            );
        }
        AppAction::WeatherUnits => {
            println!(
                "screen: WeatherUnits location={} units={}",
                model.weather.location_label(),
                model.weather.units.suffix()
            );
        }
        AppAction::MusicTogglePlayPause => {
            println!(
                "screen: MusicTogglePlayPause state={} track={}",
                model.music.state_label(),
                model.music.track_label
            );
        }
        AppAction::InternetRadioToggle => {
            println!("screen: InternetRadioToggle action=SELECT route=HTTP_MP3 audio=PCM5101_I2S");
        }
        AppAction::AssistantListenToggle => {
            println!(
                "screen: AssistantListenToggle state={} toggles={}",
                model.assistant.state_label(),
                model.assistant.toggle_count
            );
        }
        AppAction::SettingsEnter => {
            println!(
                "screen: SettingsEnter detail={}",
                model.settings.current_panel_label()
            );
        }
        AppAction::SettingsToggle => {
            if model.settings.detail_open {
                println!(
                    "screen: SettingsToggle detail={} value={}",
                    model.settings.current_panel_label(),
                    model.settings.current_value_label()
                );
            } else {
                println!(
                    "screen: SettingsPage page={}",
                    model.settings.overview_page_label()
                );
            }
        }
    }

    action
}

// RAW-R42-VIDEO-ACTION-REMOVED

// RAW-R42-R1-VIDEO-ACTION-CALLSITE-REMOVED
