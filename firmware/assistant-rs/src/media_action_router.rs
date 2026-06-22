// RAW-R54-MEDIA-ACTION-CLEANUP
// Focused Music/Internet Radio action helpers extracted after r53-r1.
// This module intentionally imports crate-root helpers to preserve accepted media behavior.
use crate::*;

pub(crate) fn audio_r33_radio_center_action(model: &mut AppState) {
    let action = internet_radio::toggle_play_stop();
    model.last_action = match action {
        "RadioPlay" => "RADIO PLAY",
        "RadioStop" => "RADIO STOP",
        "RadioNoStations" => "RADIO NO STATIONS",
        _ => "RADIO",
    };
    println!("action: {}", action);
}

pub(crate) fn audio_r33_music_center_action(model: &mut AppState) {
    let action = audio_foundation::music_toggle_play_stop();
    model.last_action = match action {
        "AudioPlay" => "AUDIO PLAY",
        "AudioStop" => "AUDIO STOP",
        "AudioMp3ProbeOnly" => "AUDIO MP3 PROBE",
        _ => "AUDIO",
    };
    println!("action: {}", action);
}

pub(crate) fn media_touch_from_summary(summary: &TouchSummary) -> media_controls::MediaTouch {
    media_controls::MediaTouch {
        end_x: summary.end_x,
        end_y: summary.end_y,
        span_x: summary.span_x,
        span_y: summary.span_y,
        dx: summary.dx,
        dy: summary.dy,
        duration_ms: summary.duration_ms,
        gesture: summary.gesture,
    }
}

pub(crate) fn audio_r33_handle_music_touch(model: &mut AppState, summary: &TouchSummary) -> bool {
    let Some(control) = media_controls::action_from_touch(media_touch_from_summary(summary)) else {
        return false;
    };

    if model.current_page == AssistantPage::InternetRadio {
        let action = match control {
            media_controls::MediaControlAction::VolumeDown => {
                model.settings.volume_percent = model.settings.volume_percent.saturating_sub(5);
                audio_foundation::set_volume_percent(model.settings.volume_percent);
                internet_radio::set_volume_percent(model.settings.volume_percent);
                "RadioVolDown"
            }
            media_controls::MediaControlAction::Previous => internet_radio::previous_station(),
            media_controls::MediaControlAction::PlayStop => internet_radio::toggle_play_stop(),
            media_controls::MediaControlAction::Next => internet_radio::next_station(),
            media_controls::MediaControlAction::VolumeUp => {
                model.settings.volume_percent =
                    model.settings.volume_percent.saturating_add(5).min(100);
                audio_foundation::set_volume_percent(model.settings.volume_percent);
                internet_radio::set_volume_percent(model.settings.volume_percent);
                "RadioVolUp"
            }
        };

        model.last_action = match action {
            "RadioPlay" => "RADIO PLAY",
            "RadioStop" => "RADIO STOP",
            "RadioNext" => "RADIO NEXT",
            "RadioPrev" => "RADIO PREV",
            "RadioVolDown" => "RADIO VOL -",
            "RadioVolUp" => "RADIO VOL +",
            "RadioNoStations" => "RADIO NO STATIONS",
            _ => "RADIO",
        };
        println!(
            "radio-r35: touch action={} zone={} x={} y={} controls=DEDICATED_ZONES audio=PCM5101_I2S",
            action,
            media_controls::action_label(control),
            summary.end_x,
            summary.end_y
        );
        println!("action: {}", action);
        return true;
    }

    if model.current_page != AssistantPage::Music {
        return false;
    }

    let action = match control {
        media_controls::MediaControlAction::VolumeDown => {
            model.settings.volume_percent = model.settings.volume_percent.saturating_sub(5);
            audio_foundation::set_volume_percent(model.settings.volume_percent);
            "AudioVolDown"
        }
        media_controls::MediaControlAction::Previous => audio_foundation::music_previous(),
        media_controls::MediaControlAction::PlayStop => audio_foundation::music_toggle_play_stop(),
        media_controls::MediaControlAction::Next => audio_foundation::music_next(),
        media_controls::MediaControlAction::VolumeUp => {
            model.settings.volume_percent =
                model.settings.volume_percent.saturating_add(5).min(100);
            audio_foundation::set_volume_percent(model.settings.volume_percent);
            "AudioVolUp"
        }
    };

    model.last_action = match action {
        "AudioPlay" => "AUDIO PLAY",
        "AudioStop" => "AUDIO STOP",
        "AudioNext" => "AUDIO NEXT",
        "AudioPrev" => "AUDIO PREV",
        "AudioVolDown" => "AUDIO VOL -",
        "AudioVolUp" => "AUDIO VOL +",
        "AudioMp3ProbeOnly" => "AUDIO MP3 PROBE",
        _ => "AUDIO",
    };
    println!(
        "audio-r35: touch action={} zone={} x={} y={} controls=DEDICATED_ZONES audio=PCM5101_I2S",
        action,
        media_controls::action_label(control),
        summary.end_x,
        summary.end_y
    );
    println!("action: {}", action);
    true
}

// RAW-R54-MOVED-MEDIA-ACTION-FUNCTIONS: audio_r33_handle_music_touch, audio_r33_music_center_action, audio_r33_radio_center_action, media_touch_from_summary
