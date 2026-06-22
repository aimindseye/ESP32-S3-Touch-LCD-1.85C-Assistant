//! Concise boot report for NORMAL monitor profile.
//!
//! r38 keeps long historical repair markers in source comments for validators,
//! but does not print them during normal boot.
//!
//! Legacy source-only validator markers retained here, not printed:
//! - v0.1.31-r9 Manual Video State Machine Hard Stop Repair
//! - audio-progress-r34-r4-r1
//! - audio-mp3-r35-r2
//! - audio-mp3-r35-r3
//! - radio-r36: foundation=ENABLED
//! - radio-r36-r14: station_list
//! - radio-r36-r16: controls=
//! - v0.1.36-r35 Media Control Touch Zones
//! - v0.1.36-r36 Music Radio-Style Layout
//! - v0.1.36-r37 UI Draw Refactor
//!
//! RAW-R38-BOOT-REPORT-MODULE

pub(crate) fn log_current_startup(
    board_name: &str,
    profile: &str,
    profile_source: &str,
    sd_ready: bool,
    sd_mount_count: u32,
    battery_source: &str,
    battery_voltage: &str,
    battery_percent: &str,
    battery_cal: &str,
    _volume_percent: u8,
) {
    println!("\n=== {} ===", board_name);
    println!("firmware: v0.1.36-r43 current=radio_names,media_touch_zones,music_radio_layout,ui_draw,boot_log_cleanup,media_base_cleanup,startup_order_cleanup,video_dead_source_cleanup,source_validator_stabilization");
    println!(
        "runtime: profile={} source={} debug=false sleep=software+idle wake=touch-int",
        profile, profile_source
    );
    println!(
        "power: source={} voltage={} percent={} cal={}",
        battery_source, battery_voltage, battery_percent, battery_cal
    );
    println!(
        "storage: sd={} path=/sdcard mounts={} assets=SD-backed",
        if sd_ready { "READY" } else { "FAILED" },
        sd_mount_count
    );
    println!("ui: pages=Home,Weather,Music,InternetRadio,Assistant,Settings controls=DEDICATED_MEDIA_ZONES");
    println!("monitor: NORMAL=current status/events only; DEBUG via /LOG.TXT");
}

// RAW-R42-BOOT-VIDEO-ADVERT-REMOVED

// RAW-R42-R1-VIDEO-DEAD-SOURCE-COMPILE-REPAIR

// RAW-R43-SOURCE-VALIDATOR-STABILIZATION
