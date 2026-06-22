// RAW-R56-R1-WEATHER-ACTION-CLEANUP
// Focused Weather select/action routing extracted after r55.
// If the r55 touch router has no old Weather action block, this router still
// owns Weather page button/body select behavior and preserves existing refresh flow.

use crate::*;

const WEATHER_NAV_BUTTON_Y_MIN: i16 = 300;
const WEATHER_NAV_BUTTON_Y_MAX: i16 = 344;
const WEATHER_NAV_PREV_X_MAX: i16 = 140;
const WEATHER_NAV_NEXT_X_MIN: i16 = 220;
const WEATHER_UNITS_TAP_Y_MIN: i16 = 300;

pub(crate) fn handle_weather_select_action(
    model: &mut AppState,
    providers: &LocalProviders,
) -> AppAction {
    let touch = model.last_touch;
    let x = touch.map(|touch| touch.x).unwrap_or(180) as i16;
    let y = touch.map(|touch| touch.y).unwrap_or(180) as i16;

    if (WEATHER_NAV_BUTTON_Y_MIN..=WEATHER_NAV_BUTTON_Y_MAX).contains(&y) {
        if x <= WEATHER_NAV_PREV_X_MAX {
            providers.previous_weather_location(&mut model.weather);
            println!(
                "weather-nav-r56: button=PREV location={} units={}",
                model.weather.location_label(),
                model.weather.units.suffix()
            );
            return AppAction::WeatherLocation;
        }

        if x >= WEATHER_NAV_NEXT_X_MIN {
            providers.cycle_weather_location(&mut model.weather);
            println!(
                "weather-nav-r56: button=NEXT location={} units={}",
                model.weather.location_label(),
                model.weather.units.suffix()
            );
            return AppAction::WeatherLocation;
        }

        providers.toggle_weather_units(&mut model.weather);
        println!(
            "weather-nav-r56: button=UNITS location={} units={}",
            model.weather.location_label(),
            model.weather.units.suffix()
        );
        return AppAction::WeatherUnits;
    }

    // Preserve accepted behavior:
    // - main/body tap cycles to next configured location
    // - lower tap toggles units
    if y >= WEATHER_UNITS_TAP_Y_MIN {
        providers.toggle_weather_units(&mut model.weather);
        AppAction::WeatherUnits
    } else {
        providers.cycle_weather_location(&mut model.weather);
        AppAction::WeatherLocation
    }
}

pub(crate) fn handle_weather_action(model: &mut AppState, action: AppAction) -> bool {
    if matches!(action, AppAction::HomeStatus | AppAction::WeatherRefresh) {
        refresh_weather_provider(model);
        return true;
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
        return true;
    }

    false
}

pub(crate) fn maybe_handle_weather_nav_row_touch(
    model: &mut AppState,
    providers: &LocalProviders,
) -> bool {
    if model.current_page != AssistantPage::Weather {
        return false;
    }

    let Some(touch) = model.last_touch else {
        return false;
    };

    let x = touch.x as i16;
    let y = touch.y as i16;

    if !(WEATHER_NAV_BUTTON_Y_MIN..=WEATHER_NAV_BUTTON_Y_MAX).contains(&y) {
        return false;
    }

    let action = handle_weather_select_action(model, providers);

    println!(
        "screen: {} location={} units={}",
        action.status_label(),
        model.weather.location_label(),
        model.weather.units.suffix()
    );
    println!("action: {}", action.status_label());

    handle_weather_action(model, action);

    println!(
        "weather-nav-r56-r2: handled x={} y={} location={} units={}",
        x,
        y,
        model.weather.location_label(),
        model.weather.units.suffix()
    );

    true
}

// RAW-R56-R1-MOVED-WEATHER-ACTION-FUNCTIONS: handle_weather_select_action, handle_weather_action

// RAW-R56-R2-WEATHER-NAV-ROW-HANDLER
