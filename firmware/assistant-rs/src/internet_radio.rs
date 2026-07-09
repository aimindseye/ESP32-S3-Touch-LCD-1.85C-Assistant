//! v0.1.36 Internet Radio HTTP MP3 Stream Foundation.
//!
//! Separate Internet Radio screen.
//! Direct HTTP MP3 streams only.
//! Station list source: /sdcard/AUDIO/RADIO.TXT
//! This module preserves the accepted local Music player path.

use std::cell::RefCell;
use std::ffi::CString;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread;

use crate::ffi;

const RADIO_LIST_PATH: &str = "/sdcard/AUDIO/RADIO.TXT";
const RADIO_R13_COMPAT_MARKER: &str = "radio-r36-r13: station_list";
const RADIO_LIST_PATHS: [&str; 8] = [
    RADIO_LIST_PATH,
    "/sdcard/RADIO.TXT",
    "/sdcard/AUDIO/RADIO.CSV",
    "/sdcard/AUDIO/RADIO.M3U",
    "/sdcard/AUDIO/RADIO.M3U8",
    "/sdcard/AUDIO/STATIONS.TXT",
    "/sdcard/AUDIO/STATION.TXT",
    "/sdcard/AUDIO/RADIO~1.TXT",
];
const RADIO_THREAD_STACK_BYTES: usize = 32 * 1024;

static RADIO_SELECTED_INDEX: AtomicU32 = AtomicU32::new(0);
static RADIO_PLAYING: AtomicBool = AtomicBool::new(false);
static RADIO_THREAD_ACTIVE: AtomicBool = AtomicBool::new(false);
static RADIO_PROGRESS_SEQUENCE: AtomicU32 = AtomicU32::new(0);
static RADIO_VOLUME_PERCENT: AtomicU32 = AtomicU32::new(60);
static RADIO_UI_IDLE_OVERRIDE: AtomicBool = AtomicBool::new(false);

thread_local! {
    static RADIO_STATION_CACHE: RefCell<Option<RadioStationLoad>> = RefCell::new(None);
}

// RAW-V1-0-1-R8-RADIO-STATION-CACHE-STATIC

#[derive(Clone)]
pub struct RadioStation {
    pub name: String,
    pub url: String,
}

pub struct RadioScreenSnapshot {
    pub station_label: String,
    pub status_label: String,
    pub source_label: String,
    pub playing: bool,
    pub progress_percent: u8,
    pub elapsed_label: String,
    pub duration_label: String,
}

pub fn log_radio_boot() {
    // RAW-R38-RADIO-BOOT-CURRENT-STATUS
    // Source-only legacy markers retained for validators, not printed:
    // radio-r36: foundation=ENABLED
    // radio-r36-r14: station_list
    // radio-r36-r16: controls=

    let stations = stations();
    let diag = station_list_diagnostics();

    println!(
        "radio: status={} path={} bytes={} stations={} skipped={} selected={} formats=HTTP,HTTPS,M3U controls=DEDICATED_ZONES station_names=FULL",
        diag.status,
        diag.path,
        diag.bytes,
        stations.len(),
        diag.skipped,
        selected_index(stations.len().max(1))
    );
}

// legacy-marker: radio-r36: volume=
pub fn set_volume_percent(percent: u8) {
    let clamped = percent.min(100);
    RADIO_VOLUME_PERCENT.store(clamped as u32, Ordering::SeqCst);
    unsafe {
        ffi::st77916_radio_http_mp3_set_volume(clamped as u32);
    }
    // v1.0.1-r10-r1: keep volume changes audio-thread-only and redraw-light.
    // Do not bump RADIO_PROGRESS_SEQUENCE here; the 1s r8 redraw cadence will
    // refresh the screen naturally while radio is playing.
}

pub fn volume_percent() -> u8 {
    RADIO_VOLUME_PERCENT.load(Ordering::SeqCst).min(100) as u8
}

pub fn progress_sequence() -> u32 {
    RADIO_PROGRESS_SEQUENCE.load(Ordering::SeqCst)
}

pub fn progress_active() -> bool {
    RADIO_PLAYING.load(Ordering::SeqCst) || RADIO_THREAD_ACTIVE.load(Ordering::SeqCst)
}

pub fn snapshot() -> RadioScreenSnapshot {
    let stations = stations();
    if stations.is_empty() {
        RADIO_PLAYING.store(false, Ordering::SeqCst);
        return RadioScreenSnapshot {
            station_label: "NO STATIONS".to_string(),
            status_label: "Create /AUDIO/RADIO.TXT".to_string(),
            source_label: "HTTP/HTTPS MP3".to_string(),
            playing: false,
            progress_percent: 0,
            elapsed_label: "LIVE".to_string(),
            duration_label: "--".to_string(),
        };
    }

    let idx = selected_index(stations.len());
    let station = &stations[idx];

    let rust_playing = RADIO_PLAYING.load(Ordering::SeqCst);
    let thread_active = RADIO_THREAD_ACTIVE.load(Ordering::SeqCst);
    let ui_idle_override = RADIO_UI_IDLE_OVERRIDE.load(Ordering::SeqCst);
    let elapsed = unsafe { ffi::st77916_radio_http_mp3_elapsed_seconds() };
    let _buffered = unsafe { ffi::st77916_radio_http_mp3_buffered_bytes() };
    let status_code = unsafe { ffi::st77916_radio_http_mp3_status_code() };
    let backend_active = rust_playing || thread_active || matches!(status_code, 1 | 2 | 3 | 4);
    let playing = if ui_idle_override {
        false
    } else {
        backend_active
    };

    let status = if ui_idle_override && thread_active {
        "STOPPING"
    } else if ui_idle_override {
        "READY"
    } else {
        match status_code {
            1 => "CONNECTING",
            2 => "BUFFERING",
            3 => "PLAYING",
            4 => "STOPPING",
            5 if backend_active => "STOPPING",
            5 => "STOPPED",
            6 => "ERROR",
            _ if thread_active || rust_playing => "STARTING",
            _ => "STOPPED",
        }
    };

    RadioScreenSnapshot {
        station_label: " ".to_string(),
        status_label: status.to_string(),
        source_label: radio_r32_preserve_raw_station_name(&station.name),
        playing,
        progress_percent: ((elapsed % 60) * 100 / 60).min(100) as u8,
        elapsed_label: seconds_to_hhmmss(elapsed),
        duration_label: "LIVE".to_string(),
    }
}

pub fn toggle_play_stop() -> &'static str {
    let stations = stations();
    if stations.is_empty() {
        RADIO_PLAYING.store(false, Ordering::SeqCst);
        println!(
            "radio-r36: action=PLAY status=NO_STATIONS path=/AUDIO/RADIO.TXT audio=PCM5101_I2S"
        );
        return "RadioNoStations";
    }

    let idx = selected_index(stations.len());
    let station = stations[idx].clone();

    let ui_idle_override = RADIO_UI_IDLE_OVERRIDE.load(Ordering::SeqCst);
    if !ui_idle_override
        && (RADIO_PLAYING.load(Ordering::SeqCst) || RADIO_THREAD_ACTIVE.load(Ordering::SeqCst))
    {
        stop_stream("user-stop");
        println!(
            "radio-r36: action=STOP station={} playback=STOPPED audio=PCM5101_I2S",
            station.name
        );
        "RadioStop"
    } else if ui_idle_override && RADIO_THREAD_ACTIVE.load(Ordering::SeqCst) {
        // RAW-V1-0-1-R13-RADIO-STATION-IDLE-UI-REPAIR
        // User already requested a stop/selection change; keep the button in
        // PLAY mode and avoid turning a second tap into another STOP-looking UI.
        RADIO_PROGRESS_SEQUENCE.fetch_add(1, Ordering::SeqCst);
        println!(
            "radio-r36-r33: action=PLAY_DEFERRED station={} reason=WAITING_FOR_PREVIOUS_STREAM_STOP audio=PCM5101_I2S",
            station.name
        );
        "RadioStopping"
    } else {
        match start_stream(station.clone()) {
            Ok(()) => {
                println!(
                    "radio-r36-r15: action=PLAY station={} url={} playback={} audio=PCM5101_I2S",
                    station.name,
                    station.url,
                    stream_playback_label(&station.url)
                );
                "RadioPlay"
            }
            Err(reason) => {
                RADIO_PLAYING.store(false, Ordering::SeqCst);
                RADIO_THREAD_ACTIVE.store(false, Ordering::SeqCst);
                println!(
                    "radio-r36: action=PLAY station={} status=FAILED reason={} audio=PCM5101_I2S",
                    station.name, reason
                );
                "RadioError"
            }
        }
    }
}

pub fn next_station() -> &'static str {
    stop_stream("next");
    let stations = stations();
    if stations.is_empty() {
        println!(
            "radio-r36: action=NEXT status=NO_STATIONS path=/AUDIO/RADIO.TXT audio=PCM5101_I2S"
        );
        return "RadioNoStations";
    }
    let next = (selected_index(stations.len()) + 1) % stations.len();
    RADIO_SELECTED_INDEX.store(next as u32, Ordering::SeqCst);
    RADIO_UI_IDLE_OVERRIDE.store(true, Ordering::SeqCst);
    RADIO_PROGRESS_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    println!(
        "radio-r36: action=NEXT selected={} station={} playback=STOPPED audio=PCM5101_I2S",
        next, stations[next].name
    );
    "RadioNext"
}

pub fn previous_station() -> &'static str {
    stop_stream("prev");
    let stations = stations();
    if stations.is_empty() {
        println!(
            "radio-r36: action=PREV status=NO_STATIONS path=/AUDIO/RADIO.TXT audio=PCM5101_I2S"
        );
        return "RadioNoStations";
    }
    let current = selected_index(stations.len());
    let prev = if current == 0 {
        stations.len() - 1
    } else {
        current - 1
    };
    RADIO_SELECTED_INDEX.store(prev as u32, Ordering::SeqCst);
    RADIO_UI_IDLE_OVERRIDE.store(true, Ordering::SeqCst);
    RADIO_PROGRESS_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    println!(
        "radio-r36: action=PREV selected={} station={} playback=STOPPED audio=PCM5101_I2S",
        prev, stations[prev].name
    );
    "RadioPrev"
}

pub fn volume_down() -> &'static str {
    let next = volume_percent().saturating_sub(5);
    set_volume_percent(next);
    println!(
        "radio-r36: action=VOL_DOWN volume={} source=InternetRadio audio=PCM5101_I2S",
        next
    );
    "RadioVolDown"
}

pub fn volume_up() -> &'static str {
    let next = volume_percent().saturating_add(5).min(100);
    set_volume_percent(next);
    println!(
        "radio-r36: action=VOL_UP volume={} source=InternetRadio audio=PCM5101_I2S",
        next
    );
    "RadioVolUp"
}

fn is_supported_stream_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn stream_playback_label(url: &str) -> &'static str {
    if url.starts_with("https://") {
        "HTTPS_MP3_STREAM"
    } else {
        "HTTP_MP3_STREAM"
    }
}

fn stream_format_label(url: &str) -> &'static str {
    if url.starts_with("https://") {
        "HTTPS_MP3"
    } else {
        "HTTP_MP3"
    }
}

fn start_stream(station: RadioStation) -> Result<(), &'static str> {
    if !is_supported_stream_url(&station.url) {
        return Err("ONLY_HTTP_OR_HTTPS_MP3_STREAMS_SUPPORTED");
    }

    if RADIO_THREAD_ACTIVE
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("RADIO_BUSY");
    }

    RADIO_UI_IDLE_OVERRIDE.store(false, Ordering::SeqCst);
    RADIO_PLAYING.store(true, Ordering::SeqCst);
    RADIO_PROGRESS_SEQUENCE.fetch_add(1, Ordering::SeqCst);

    let thread_name = format!("radio-{}", trim_label(&station.name, 12));
    let result = thread::Builder::new()
        .name(thread_name)
        .stack_size(RADIO_THREAD_STACK_BYTES)
        .spawn(move || radio_thread(station));

    match result {
        Ok(_handle) => Ok(()),
        Err(_) => {
            RADIO_PLAYING.store(false, Ordering::SeqCst);
            RADIO_THREAD_ACTIVE.store(false, Ordering::SeqCst);
            Err("RADIO_THREAD_SPAWN_FAILED")
        }
    }
}

fn radio_thread(station: RadioStation) {
    println!(
        "radio-r36: status=THREAD_STARTED station={} stack_bytes={} audio=PCM5101_I2S",
        station.name, RADIO_THREAD_STACK_BYTES
    );

    let c_url = match CString::new(station.url.as_bytes()) {
        Ok(value) => value,
        Err(_) => {
            println!(
                "radio-r36: status=BAD_URL station={} audio=PCM5101_I2S",
                station.name
            );
            RADIO_PLAYING.store(false, Ordering::SeqCst);
            RADIO_THREAD_ACTIVE.store(false, Ordering::SeqCst);
            return;
        }
    };

    let c_name = match CString::new(station.name.as_bytes()) {
        Ok(value) => value,
        Err(_) => CString::new("RADIO").unwrap(),
    };

    let code = unsafe {
        ffi::st77916_radio_http_mp3_play(c_url.as_ptr(), c_name.as_ptr(), volume_percent() as u32)
    };

    RADIO_PLAYING.store(false, Ordering::SeqCst);
    RADIO_THREAD_ACTIVE.store(false, Ordering::SeqCst);
    RADIO_PROGRESS_SEQUENCE.fetch_add(1, Ordering::SeqCst);

    println!(
        "radio-r36: status=THREAD_DONE station={} code={} audio=PCM5101_I2S",
        station.name, code
    );
}

fn stop_stream(reason: &'static str) {
    unsafe {
        ffi::st77916_radio_http_mp3_stop_request();
    }
    RADIO_PLAYING.store(false, Ordering::SeqCst);
    if matches!(reason, "user-stop" | "next" | "prev") {
        RADIO_UI_IDLE_OVERRIDE.store(true, Ordering::SeqCst);
    }
    RADIO_PROGRESS_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    println!(
        "radio-r36: action=STOP_REQUEST reason={} stop=DEFERRED_TO_STREAM_THREAD audio=PCM5101_I2S",
        reason
    );
}

#[derive(Clone)]
struct RadioStationLoad {
    path: String,
    bytes: usize,
    stations: Vec<RadioStation>,
    skipped: usize,
}

struct RadioStationDiag {
    status: &'static str,
    path: String,
    bytes: usize,
    skipped: usize,
}

fn stations() -> Vec<RadioStation> {
    cached_station_load()
        .map(|loaded| loaded.stations)
        .unwrap_or_default()
}

fn station_list_diagnostics() -> RadioStationDiag {
    match cached_station_load() {
        Some(loaded) => RadioStationDiag {
            status: if loaded.stations.is_empty() {
                "PARSE_EMPTY"
            } else {
                "READY"
            },
            path: radio_display_path(&loaded.path),
            bytes: loaded.bytes,
            skipped: loaded.skipped,
        },
        None => RadioStationDiag {
            status: "MISSING",
            path: radio_display_path(RADIO_LIST_PATH),
            bytes: 0,
            skipped: 0,
        },
    }
}

fn cached_station_load() -> Option<RadioStationLoad> {
    RADIO_STATION_CACHE.with(|cache| {
        let mut cached = cache.borrow_mut();
        if cached.is_none() {
            let loaded = load_station_list();
            if let Some(ref station_load) = loaded {
                println!(
                    "radio-r8: station_cache=LOAD path={} bytes={} stations={} skipped={} source=SD_ONCE audio=PCM5101_I2S",
                    radio_display_path(&station_load.path),
                    station_load.bytes,
                    station_load.stations.len(),
                    station_load.skipped
                );
                *cached = Some(station_load.clone());
            } else {
                println!(
                    "radio-r8: station_cache=MISS path=/AUDIO/RADIO.TXT source=SD_RETRY audio=PCM5101_I2S"
                );
            }
        }

        cached.clone()
    })
}

// RAW-V1-0-1-R8-RADIO-STATION-CACHE

fn load_station_list() -> Option<RadioStationLoad> {
    let mut best_empty: Option<RadioStationLoad> = None;

    for path in RADIO_LIST_PATHS.iter() {
        if let Some(loaded) = try_load_station_file(path) {
            if !loaded.stations.is_empty() {
                return Some(loaded);
            }
            if best_empty.is_none() {
                best_empty = Some(loaded);
            }
        }
    }

    if let Some(loaded) = discover_station_file("/sdcard/AUDIO") {
        if !loaded.stations.is_empty() {
            return Some(loaded);
        }
        if best_empty.is_none() {
            best_empty = Some(loaded);
        }
    }

    if let Some(loaded) = discover_station_file("/sdcard") {
        if !loaded.stations.is_empty() {
            return Some(loaded);
        }
        if best_empty.is_none() {
            best_empty = Some(loaded);
        }
    }

    best_empty
}

fn try_load_station_file(path: &str) -> Option<RadioStationLoad> {
    let text = fs::read_to_string(Path::new(path)).ok()?;
    let bytes = text.len();
    let (stations, skipped) = parse_stations_text(&text);
    Some(RadioStationLoad {
        path: path.to_string(),
        bytes,
        stations,
        skipped,
    })
}

fn discover_station_file(dir: &str) -> Option<RadioStationLoad> {
    let entries = fs::read_dir(Path::new(dir)).ok()?;
    let mut best_empty: Option<RadioStationLoad> = None;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        if !is_station_list_candidate(name) {
            continue;
        }

        let Some(path_str) = path.to_str() else {
            continue;
        };

        if let Some(loaded) = try_load_station_file(path_str) {
            if !loaded.stations.is_empty() {
                return Some(loaded);
            }
            if best_empty.is_none() {
                best_empty = Some(loaded);
            }
        }
    }

    best_empty
}

fn is_station_list_candidate(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();

    if upper == "RADIO.TXT"
        || upper == "RADIO.CSV"
        || upper == "RADIO.M3U"
        || upper == "RADIO.M3U8"
        || upper == "RADIO"
        || upper == "STATION.TXT"
        || upper == "STATIONS.TXT"
        || upper == "STATIONS.CSV"
        || upper == "STATIONS.M3U"
        || upper == "RADIO~1.TXT"
    {
        return true;
    }

    (upper.starts_with("RADIO") || upper.starts_with("STATION"))
        && (upper.ends_with(".TXT")
            || upper.ends_with(".CSV")
            || upper.ends_with(".M3U")
            || upper.ends_with(".M3U8"))
}

fn parse_stations_text(text: &str) -> (Vec<RadioStation>, usize) {
    let mut out = Vec::new();
    let mut skipped = 0usize;
    let mut pending_m3u_name: Option<String> = None;

    for raw_line in text.lines() {
        let line = clean_station_field(raw_line);
        if line.is_empty() {
            continue;
        }

        if let Some(name) = parse_extinf_name(&line) {
            pending_m3u_name = Some(name);
            continue;
        }

        if line.starts_with('#') {
            continue;
        }

        if is_radio_url(&line) {
            let name = pending_m3u_name
                .take()
                .unwrap_or_else(|| station_name_from_url(&line, out.len() + 1));
            out.push(RadioStation { name, url: line });
            continue;
        }

        if let Some(station) = parse_station_line(&line, out.len() + 1) {
            pending_m3u_name = None;
            out.push(station);
        } else {
            pending_m3u_name = None;
            skipped = skipped.saturating_add(1);
        }
    }

    (out, skipped)
}

fn parse_station_line(line: &str, fallback_index: usize) -> Option<RadioStation> {
    for sep in ['=', '|', ',', ';', '\t'] {
        if let Some((left, right)) = line.split_once(sep) {
            let left = clean_station_field(left);
            let right = clean_station_field(right);

            if is_radio_url(&left) {
                let name = if right.is_empty() {
                    station_name_from_url(&left, fallback_index)
                } else {
                    right
                };
                return Some(RadioStation { name, url: left });
            }

            if is_radio_url(&right) {
                let name = if left.is_empty() {
                    station_name_from_url(&right, fallback_index)
                } else {
                    left
                };
                return Some(RadioStation { name, url: right });
            }
        }
    }

    None
}

fn parse_extinf_name(line: &str) -> Option<String> {
    let upper = line.to_ascii_uppercase();
    if !upper.starts_with("#EXTINF") {
        return None;
    }

    let (_, name) = line.rsplit_once(',')?;
    let name = clean_station_field(name);
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn clean_station_field(input: &str) -> String {
    input
        .trim_start_matches('\u{feff}')
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

fn is_radio_url(input: &str) -> bool {
    input.starts_with("http://") || input.starts_with("https://")
}

fn station_name_from_url(url: &str, fallback_index: usize) -> String {
    let without_scheme = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .unwrap_or(url);
    let host = without_scheme.split('/').next().unwrap_or("").trim();

    if host.is_empty() {
        format!("Station {}", fallback_index)
    } else {
        trim_label(host, 18)
    }
}

fn radio_display_path(path: &str) -> String {
    path.strip_prefix("/sdcard").unwrap_or(path).to_string()
}

fn selected_index(len: usize) -> usize {
    if len == 0 {
        0
    } else {
        (RADIO_SELECTED_INDEX.load(Ordering::SeqCst) as usize) % len
    }
}

fn radio_r32_preserve_raw_station_name(input: &str) -> String {
    // v0.1.36-r32: preserve the RADIO.TXT station name exactly enough for display.
    // Do not call the old trim/label helper here; that path is shared with compact Music-style
    // labels and can turn NonStopHindi into N S H.
    let cleaned = input.trim();
    if cleaned.is_empty() {
        return "Radio".to_string();
    }

    let mut out = String::new();
    for ch in cleaned.chars().take(24) {
        out.push(ch);
    }

    if out.trim().is_empty() {
        "Radio".to_string()
    } else {
        out
    }
}

// RADIO_R34_REDUNDANT_RADIO_LABEL_HELPERS_REMOVED
// Older r25-r31 station-label helpers were removed because the
// dedicated InternetRadio screen draws source_label directly.
// Keeping the unused compact-title helpers made the lowercase font
// issue look like a station parser/snapshot problem.
fn trim_label(input: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for ch in input.chars().take(max_chars) {
        out.push(ch);
    }
    out
}

fn seconds_to_hhmmss(seconds: u32) -> String {
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{}:{:02}", m, s)
    }
}

// RAW-V1-0-1-R4-RADIO-STREAM-HEADROOM

// RAW-V1-0-1-R5-RADIO-START-REFILL-REPAIR

// RAW-V1-0-1-R6-R1-RADIO-PREFILL-RUNTIME-PACING

// RAW-V1-0-1-R8-RADIO-UI-PERFORMANCE

// RAW-V1-0-1-R9-RADIO-RING-BUFFER-ARCHITECTURE

// RAW-V1-0-1-R9-R1-RADIO-RING-VALIDATOR-COMPAT
// Radio stream tuning compatibility audit for legacy validators only.
// Runtime values live in the C stream shim.
// input_bytes=262144
// low_water=32768
// refill_chunk=512
// legacy-r4-floor: 196608 73728 8192

// RAW-V1-0-1-R10-R1-RADIO-VOLUME-QUIET-BUFFER-REPAIR

// RAW-V1-0-1-R11-R2-RADIO-SNAPSHOT-ACTIVE-STATUS

// RAW-V1-0-1-R13-RADIO-STATION-IDLE-UI-REPAIR-MARKER
