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

fn radio_r7_center_layout_action_from_touch(
    summary: &TouchSummary,
) -> Option<media_controls::MediaControlAction> {
    let x = summary.end_x as i32;
    let y = summary.end_y as i32;

    // v1.0.1-r7: Internet Radio no longer uses side-edge volume zones.
    // This intentionally leaves shared Music media_controls unchanged.
    if x < 30 || x > 360 {
        return None;
    }

    // Volume row above transport controls.
    if (164..=216).contains(&y) {
        if (54..=150).contains(&x) {
            return Some(media_controls::MediaControlAction::VolumeDown);
        }
        if (230..=336).contains(&x) {
            return Some(media_controls::MediaControlAction::VolumeUp);
        }
        return None;
    }

    // Transport row.
    if (224..=286).contains(&y) {
        if (44..=128).contains(&x) {
            return Some(media_controls::MediaControlAction::Previous);
        }
        if (134..=246).contains(&x) {
            return Some(media_controls::MediaControlAction::PlayStop);
        }
        if (252..=346).contains(&x) {
            return Some(media_controls::MediaControlAction::Next);
        }
    }

    None
}

pub(crate) fn audio_r33_handle_music_touch(model: &mut AppState, summary: &TouchSummary) -> bool {
    if model.current_page == AssistantPage::InternetRadio {
        let Some(control) = radio_r7_center_layout_action_from_touch(summary) else {
            return false;
        };

        let action = match control {
            media_controls::MediaControlAction::VolumeDown => {
                model.settings.volume_percent = model.settings.volume_percent.saturating_sub(5);
                internet_radio::set_volume_percent(model.settings.volume_percent);
                "RadioVolDown"
            }
            media_controls::MediaControlAction::Previous => internet_radio::previous_station(),
            media_controls::MediaControlAction::PlayStop => internet_radio::toggle_play_stop(),
            media_controls::MediaControlAction::Next => internet_radio::next_station(),
            media_controls::MediaControlAction::VolumeUp => {
                model.settings.volume_percent =
                    model.settings.volume_percent.saturating_add(5).min(100);
                internet_radio::set_volume_percent(model.settings.volume_percent);
                "RadioVolUp"
            }
        };

        model.last_action = match action {
            "RadioPlay" => "RADIO PLAY",
            "RadioStop" => "RADIO STOP",
            "RadioStopping" => "RADIO STOPPING",
            "RadioNext" => "RADIO NEXT",
            "RadioPrev" => "RADIO PREV",
            "RadioVolDown" => "RADIO VOL -",
            "RadioVolUp" => "RADIO VOL +",
            "RadioNoStations" => "RADIO NO STATIONS",
            _ => "RADIO",
        };
        if !matches!(
            control,
            media_controls::MediaControlAction::VolumeDown
                | media_controls::MediaControlAction::VolumeUp
        ) {
            println!(
                "radio-r35: touch action={} zone={} x={} y={} controls=DEDICATED_ZONES audio=PCM5101_I2S",
                action,
                media_controls::action_label(control),
                summary.end_x,
                summary.end_y
            );
            println!("action: {}", action);
        }
        return true;
    }

    if model.current_page != AssistantPage::Music {
        return false;
    }

    let Some(control) = media_controls::action_from_touch(media_touch_from_summary(summary)) else {
        return false;
    };

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

// RAW-V1-0-1-R4-RADIO-STREAM-HEADROOM

// RAW-V1-0-1-R7-INTERNET-RADIO-CENTER-VOLUME-LAYOUT

// RAW-V1-0-1-R7-R1-INTERNET-RADIO-READABILITY-REPAIR

// RAW-V1-0-1-R10-R1-RADIO-VOLUME-QUIET-BUFFER-REPAIR

// RAW-V1-0-1-R13-RADIO-STATION-IDLE-UI-REPAIR
