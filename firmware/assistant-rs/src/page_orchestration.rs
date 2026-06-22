// RAW-R51-MAIN-ORCHESTRATION-CLEANUP
// Page dispatch/navigation glue moved out of main.rs after r50.
// This module intentionally imports crate-root helpers to preserve runtime behavior.
use crate::*;

pub(crate) fn handle_intent(
    model: &mut AppState,
    providers: &LocalProviders,
    wifi: &mut EspWifi<'static>,
    wifi_connect_deadline: &mut Option<Instant>,
    intent: UiIntent,
) {
    match intent {
        UiIntent::NextPage => {
            model.next_page();
            println!("nav: NextPage -> {:?}", model.current_page);
        }
        UiIntent::PreviousPage => {
            model.previous_page();
            println!("nav: PreviousPage -> {:?}", model.current_page);
        }
        UiIntent::Select => {
            model.note_intent(UiIntent::Select);
            if model.current_page == AssistantPage::InternetRadio {
                media_action_router::audio_r33_radio_center_action(model);
                return;
            }
            if model.current_page == AssistantPage::Music {
                media_action_router::audio_r33_music_center_action(model);
                return;
            }
            let action = handle_select_action(model, providers);
            println!("{}", action.log_marker());

            if action == AppAction::SettingsToggle
                && model.current_page == AssistantPage::Settings
                && model.settings.selected == SettingsPanel::Network
                && model.settings.take_wifi_connect_request()
            {
                start_wifi_credential_import_and_connect(model, wifi, wifi_connect_deadline);
            }

            if action == AppAction::SettingsToggle
                && model.current_page == AssistantPage::Settings
                && model.settings.detail_open
                && model.settings.selected == SettingsPanel::Sound
            {
                audio_foundation::set_volume_percent(model.settings.volume_percent);
                println!(
                    "audio-settings-r34: action=VOLUME volume={} route=PCM5101_I2S audio=PCM5101_I2S",
                    model.settings.volume_percent
                );
            }

            if matches!(action, AppAction::HomeStatus | AppAction::WeatherRefresh) {
                refresh_weather_provider(model);
            }

            if matches!(action, AppAction::WeatherLocation | AppAction::WeatherUnits)
                || (action == AppAction::SettingsToggle
                    && model.current_page == AssistantPage::Settings
                    && model.settings.detail_open
                    && model.settings.selected == SettingsPanel::Weather)
            {
                println!(
                    "weather-config: changed location={} units={} action={}",
                    model.weather.location_label(),
                    model.weather.units.suffix(),
                    action.status_label()
                );
                refresh_weather_provider(model);
            }
        }
        UiIntent::SettingsBackToOverview => {
            model.note_intent(UiIntent::SettingsBackToOverview);
            providers.close_settings_detail(&mut model.settings);
            println!(
                "settings-nav: detail back -> overview selected={}",
                model.settings.current_panel_label()
            );
            println!(
                "screen: SettingsBack overview selected={}",
                model.settings.current_panel_label()
            );
        }
        UiIntent::SettingsNextDetail => {
            model.note_intent(UiIntent::SettingsNextDetail);
            providers.next_settings_detail(&mut model.settings);
            println!(
                "settings-nav: next detail -> {}",
                model.settings.current_panel_label()
            );
        }
        UiIntent::SettingsPreviousDetail => {
            model.note_intent(UiIntent::SettingsPreviousDetail);
            providers.previous_settings_detail(&mut model.settings);
            println!(
                "settings-nav: previous detail -> {}",
                model.settings.current_panel_label()
            );
        }
        UiIntent::BackHome => {
            model.set_page(AssistantPage::Home);
            model.note_intent(UiIntent::BackHome);
            println!("nav: BackHome -> Home");
        }
        UiIntent::AssistantHold => {
            model.note_intent(UiIntent::AssistantHold);
            println!("action: Assistant placeholder");
        }
        UiIntent::BootReserved => {
            model.note_intent(UiIntent::BootReserved);
            println!("action: BOOT reserved");
        }
        UiIntent::PowerMenu => {
            model.note_intent(UiIntent::PowerMenu);
            println!("action: Power menu placeholder");
        }
    }
}

pub(crate) fn accent_for_page(page: AssistantPage) -> u16 {
    match page {
        AssistantPage::Home => ACCENT_HOME,
        AssistantPage::Weather => ACCENT_WEATHER,
        AssistantPage::Music => ACCENT_MUSIC,
        AssistantPage::InternetRadio => ACCENT_MUSIC,
        AssistantPage::Assistant => ACCENT_ASSISTANT,
        AssistantPage::Settings => ACCENT_SETTINGS,
    }
}

// RAW-R51-MOVED-FUNCTIONS: draw_page_fallback_base, draw_cached_page_base, process_touch_summary, handle_intent, accent_for_page

// RAW-R51-R1-CACHED-PAGE-BASE-EXPORTED
