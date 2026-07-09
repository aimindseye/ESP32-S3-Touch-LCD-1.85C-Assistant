//! Concise boot report for the accepted v1.0.1-r13 firmware line.

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
    println!("firmware: v1.0.1-r13 current=touch_guard,weather_fahrenheit,music_mp3,internet_radio_streambuffer,radio_live_ui,source_cleanup");
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
    println!("audio: output=PCM5101_I2S decoder=HELIX radio=PSRAM_STREAMBUFFER");
    println!("monitor: NORMAL=current status/events only; DEBUG via /LOG.TXT");
}

// RAW-V1-0-1-R14-CLEAN-BOOT-REPORT
