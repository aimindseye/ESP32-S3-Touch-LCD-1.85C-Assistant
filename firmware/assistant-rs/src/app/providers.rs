//! Local provider boundary.
//!
//! v0.1.16 keeps all integrations mocked/local, while Settings gains local
//! functional subscreens behind the provider boundary.

use crate::app::{
    assistant::AssistantState,
    home::HomeState,
    music::MusicState,
    settings::{SettingsPanel, SettingsState},
    weather::WeatherState,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalProviders;

impl LocalProviders {
    pub const fn new() -> Self {
        Self
    }

    pub fn toggle_home_status(&self, state: &mut HomeState) {
        state.refresh_glance();
    }

    pub fn refresh_weather(&self, state: &mut WeatherState) {
        state.request_fetch();
    }

    pub fn cycle_weather_location(&self, state: &mut WeatherState) {
        state.cycle_location();
    }

    pub fn previous_weather_location(&self, state: &mut WeatherState) {
        state.previous_location();
    }

    pub fn toggle_weather_units(&self, state: &mut WeatherState) {
        state.toggle_units();
    }

    pub fn toggle_music_play_pause(&self, state: &mut MusicState) {
        state.toggle_play_pause();
    }

    pub fn toggle_assistant_listening(&self, state: &mut AssistantState) {
        state.toggle_listening();
    }

    pub fn enter_settings_detail(&self, state: &mut SettingsState, panel: SettingsPanel) {
        state.enter_detail_for(panel);
    }

    pub fn next_settings_overview_page(&self, state: &mut SettingsState) {
        state.next_overview_page();
    }

    pub fn close_settings_detail(&self, state: &mut SettingsState) {
        state.close_detail();
    }

    pub fn next_settings_detail(&self, state: &mut SettingsState) {
        state.next_detail_panel();
    }

    pub fn previous_settings_detail(&self, state: &mut SettingsState) {
        state.previous_detail_panel();
    }

    pub fn toggle_settings_current(&self, state: &mut SettingsState) {
        state.toggle_current();
    }
}

pub const PROVIDER_BOUNDARY_MARKER: &str =
    "v0.1.22 provider boundary: Home refresh plus Settings details hub";

// RAW-R56-WEATHER-PREVIOUS-PROVIDER

// RAW-R56-R1-WEATHER-PREVIOUS-PROVIDER
