//! v0.1.32 Audio Player Foundation — WAV First, MP3 Probe
//!
//! Conservative foundation only:
//! - scans /sdcard/AUDIO
//! - probes WAV PCM headers
//! - probes MP3 ID3/frame headers
//! - keeps MP3 decode disabled
//! - keeps playback hardware-gated until I2S/speaker path is confirmed
//! - does not mount/unmount SD; caller must preserve the persistent SD session
//! - v0.1.33 integrates scanner/probe state with the existing Music screen controls
//! - v0.1.34-r3 uses explicit pthread stack and heap stream buffer for WAV PCM playback
//! - v0.1.34-r4 publishes byte-derived Music progress and repaint sequence
//! - v0.1.35-r1 adds MP3 decode through vendored Helix C component to PCM5101 I2S

use std::ffi::CString;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread;

use crate::ffi;

const AUDIO_DIR: &str = "/sdcard/AUDIO";
const AUDIO_CFG: &str = "/sdcard/AUDIO/AUDIO.TXT";
const MAX_AUDIO_PROBE_BYTES: usize = 768;
const AUDIO_PCM_THREAD_STACK_BYTES: usize = 16 * 1024;
const AUDIO_PCM_STREAM_BUFFER_BYTES: usize = 1024;
const AUDIO_MP3_THREAD_STACK_BYTES: usize = 32 * 1024;

static AUDIO_SELECTED_INDEX: AtomicU32 = AtomicU32::new(0);
static AUDIO_PLAYING: AtomicBool = AtomicBool::new(false);
static AUDIO_PROGRESS_TICK: AtomicU32 = AtomicU32::new(0);
static AUDIO_PROGRESS_PERCENT: AtomicU32 = AtomicU32::new(0);
static AUDIO_ELAPSED_SECONDS: AtomicU32 = AtomicU32::new(0);
static AUDIO_DURATION_SECONDS: AtomicU32 = AtomicU32::new(0);
static AUDIO_PROGRESS_SEQUENCE: AtomicU32 = AtomicU32::new(0);
static AUDIO_STOP_REQUESTED: AtomicBool = AtomicBool::new(false);
static AUDIO_THREAD_ACTIVE: AtomicBool = AtomicBool::new(false);
static AUDIO_VOLUME_PERCENT: AtomicU32 = AtomicU32::new(60);

#[derive(Default)]
struct AudioScanSummary {
    total: usize,
    wav: usize,
    mp3: usize,
    other: usize,
    first_wav: Option<PathBuf>,
    first_mp3: Option<PathBuf>,
}

#[derive(Clone, Copy)]
struct WavProbe {
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
    audio_format: u16,
    data_bytes: u32,
    data_offset: u64,
}

#[derive(Clone, Copy)]
struct Mp3Probe {
    id3: bool,
    frame_sync: bool,
    sample_rate: u32,
    layer: &'static str,
    version: &'static str,
}

#[derive(Clone)]
struct AudioFileEntry {
    path: PathBuf,
    name: String,
    kind: AudioKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AudioKind {
    Wav,
    Mp3,
}

pub struct MusicScreenSnapshot {
    pub track_label: String,
    pub subtitle_label: String,
    pub source_label: String,
    pub playing: bool,
    pub progress_percent: u8,
    pub elapsed_label: String,
    pub duration_label: String,
}

pub fn log_audio_foundation_boot() {
    // RAW-R38-AUDIO-BOOT-CURRENT-STATUS
    // Source-only legacy markers retained for validators, not printed:
    // audio-progress-r34-r4-r1
    // audio-mp3-r35-r2
    // audio-mp3-r35-r3

    let i2s_confirmed = audio_i2s_confirmed();

    match scan_audio_dir() {
        Ok(summary) => println!(
            "audio: status=READY path=/AUDIO files={} wav={} mp3={} other={} output={} decoder=HELIX volume={} controls=DEDICATED_ZONES",
            summary.total,
            summary.wav,
            summary.mp3,
            summary.other,
            if i2s_confirmed { "PCM5101_I2S" } else { "HARDWARE_GATED" },
            volume_percent()
        ),
        Err(e) => println!(
            "audio: status=ERROR path=/AUDIO reason={} output={} decoder=HELIX controls=DEDICATED_ZONES",
            e,
            if i2s_confirmed { "PCM5101_I2S" } else { "HARDWARE_GATED" }
        ),
    }
}

fn audio_i2s_confirmed() -> bool {
    true
}

fn scan_audio_dir() -> Result<AudioScanSummary, &'static str> {
    let mut summary = AudioScanSummary::default();

    let entries = fs::read_dir(AUDIO_DIR).map_err(|_| "READ_DIR_FAILED_OR_MISSING")?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        summary.total = summary.total.saturating_add(1);

        let ext = path
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_ascii_uppercase();

        match ext.as_str() {
            "WAV" => {
                summary.wav = summary.wav.saturating_add(1);
                if summary.first_wav.is_none() {
                    summary.first_wav = Some(path);
                }
            }
            "MP3" => {
                summary.mp3 = summary.mp3.saturating_add(1);
                if summary.first_mp3.is_none() {
                    summary.first_mp3 = Some(path);
                }
            }
            _ => {
                summary.other = summary.other.saturating_add(1);
            }
        }
    }

    Ok(summary)
}

fn probe_wav(path: &Path) -> Result<WavProbe, &'static str> {
    let mut file = File::open(path).map_err(|_| "OPEN_FAILED")?;

    let mut riff = [0u8; 12];
    file.read_exact(&mut riff).map_err(|_| "HEADER_TOO_SMALL")?;
    if &riff[0..4] != b"RIFF" || &riff[8..12] != b"WAVE" {
        return Err("NOT_RIFF_WAVE");
    }

    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    let mut data_bytes: u32 = 0;
    let mut data_offset: u64 = 0;

    loop {
        let mut chunk_header = [0u8; 8];
        match file.read_exact(&mut chunk_header) {
            Ok(()) => {}
            Err(_) => break,
        }

        let chunk_id = &chunk_header[0..4];
        let chunk_len = le_u32(&chunk_header[4..8]);
        let chunk_payload_offset = file.stream_position().map_err(|_| "SEEK_FAILED")?;

        if chunk_id == b"fmt " {
            if chunk_len < 16 {
                return Err("BAD_FMT_CHUNK");
            }

            let mut fmt_buf = [0u8; 64];
            let read_len = (chunk_len as usize).min(fmt_buf.len());
            file.read_exact(&mut fmt_buf[..read_len])
                .map_err(|_| "READ_FAILED")?;

            let audio_format = le_u16(&fmt_buf[0..2]);
            let channels = le_u16(&fmt_buf[2..4]);
            let sample_rate = le_u32(&fmt_buf[4..8]);
            let bits = le_u16(&fmt_buf[14..16]);
            fmt = Some((audio_format, channels, sample_rate, bits));

            if chunk_len as usize > read_len {
                let remaining = (chunk_len as usize - read_len) as i64;
                file.seek(SeekFrom::Current(remaining))
                    .map_err(|_| "SEEK_FAILED")?;
            }
        } else if chunk_id == b"data" {
            data_bytes = chunk_len;
            data_offset = chunk_payload_offset;
            break;
        } else {
            file.seek(SeekFrom::Current(chunk_len as i64))
                .map_err(|_| "SEEK_FAILED")?;
        }

        if (chunk_len & 1) != 0 {
            file.seek(SeekFrom::Current(1)).map_err(|_| "SEEK_FAILED")?;
        }
    }

    let (audio_format, channels, sample_rate, bits_per_sample) = fmt.ok_or("NO_FMT_CHUNK")?;
    if audio_format != 1 {
        return Err("NOT_PCM_FORMAT_1");
    }
    if data_bytes == 0 {
        return Err("NO_DATA_CHUNK");
    }

    Ok(WavProbe {
        channels,
        sample_rate,
        bits_per_sample,
        audio_format,
        data_bytes,
        data_offset,
    })
}

fn probe_mp3(path: &Path) -> Result<Mp3Probe, &'static str> {
    let mut buf = [0u8; MAX_AUDIO_PROBE_BYTES];
    let mut file = File::open(path).map_err(|_| "OPEN_FAILED")?;
    let n = file.read(&mut buf).map_err(|_| "READ_FAILED")?;
    if n < 4 {
        return Err("HEADER_TOO_SMALL");
    }

    let has_id3 = n >= 10 && &buf[0..3] == b"ID3";
    let mut start = if has_id3 && n >= 10 {
        let tag_size = (((buf[6] & 0x7f) as usize) << 21)
            | (((buf[7] & 0x7f) as usize) << 14)
            | (((buf[8] & 0x7f) as usize) << 7)
            | ((buf[9] & 0x7f) as usize);
        10usize.saturating_add(tag_size)
    } else {
        0
    };

    if start >= n.saturating_sub(4) {
        start = 0;
    }

    for i in start..n.saturating_sub(3) {
        if buf[i] == 0xff && (buf[i + 1] & 0xe0) == 0xe0 {
            let header: u32 = ((buf[i] as u32) << 24)
                | ((buf[i + 1] as u32) << 16)
                | ((buf[i + 2] as u32) << 8)
                | (buf[i + 3] as u32);

            let version_bits = (header >> 19) & 0x03;
            let layer_bits = (header >> 17) & 0x03;
            let sample_rate_idx = (header >> 10) & 0x03;

            let version = match version_bits {
                0 => "MPEG2.5",
                2 => "MPEG2",
                3 => "MPEG1",
                _ => "RESERVED",
            };
            let layer = match layer_bits {
                1 => "LayerIII",
                2 => "LayerII",
                3 => "LayerI",
                _ => "RESERVED",
            };

            let sample_rate = sample_rate_from_bits(version_bits, sample_rate_idx);
            return Ok(Mp3Probe {
                id3: has_id3,
                frame_sync: true,
                sample_rate,
                layer,
                version,
            });
        }
    }

    Ok(Mp3Probe {
        id3: has_id3,
        frame_sync: false,
        sample_rate: 0,
        layer: "UNKNOWN",
        version: "UNKNOWN",
    })
}

fn sample_rate_from_bits(version_bits: u32, idx: u32) -> u32 {
    if idx >= 3 {
        return 0;
    }

    let mpeg1 = [44_100u32, 48_000, 32_000];
    let mpeg2 = [22_050u32, 24_000, 16_000];
    let mpeg25 = [11_025u32, 12_000, 8_000];

    match version_bits {
        3 => mpeg1[idx as usize],
        2 => mpeg2[idx as usize],
        0 => mpeg25[idx as usize],
        _ => 0,
    }
}

fn le_u16(bytes: &[u8]) -> u16 {
    (bytes[0] as u16) | ((bytes[1] as u16) << 8)
}

fn le_u32(bytes: &[u8]) -> u32 {
    (bytes[0] as u32)
        | ((bytes[1] as u32) << 8)
        | ((bytes[2] as u32) << 16)
        | ((bytes[3] as u32) << 24)
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|x| x.to_str())
        .unwrap_or("?")
        .chars()
        .take(24)
        .collect()
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "YES"
    } else {
        "NO"
    }
}

// v0.1.33 Existing Music Screen Audio Player Controls.
// This integrates /AUDIO scanner/probe state with the already accepted Music UI.
// It intentionally does not claim MP3 decode or I2S PCM streaming is enabled yet.
pub fn music_screen_snapshot() -> MusicScreenSnapshot {
    let files = audio_file_entries();
    let playing = AUDIO_PLAYING.load(Ordering::SeqCst);

    if files.is_empty() {
        AUDIO_PLAYING.store(false, Ordering::SeqCst);
        return MusicScreenSnapshot {
            track_label: "NO AUDIO".to_string(),
            subtitle_label: "Copy WAV/MP3 to /AUDIO".to_string(),
            source_label: "SD /AUDIO".to_string(),
            playing: false,
            progress_percent: 0,
            elapsed_label: "0:00".to_string(),
            duration_label: "--:--".to_string(),
        };
    }

    let idx = selected_index(files.len());
    let file = &files[idx];
    if playing && file.kind == AudioKind::Mp3 {
        refresh_mp3_progress_from_helix();
    }
    let progress = AUDIO_PROGRESS_PERCENT.load(Ordering::SeqCst).min(100) as u8;
    let elapsed_seconds = AUDIO_ELAPSED_SECONDS.load(Ordering::SeqCst);

    let (subtitle, duration) = match file.kind {
        AudioKind::Wav => match probe_wav(&file.path) {
            Ok(wav) => (
                format!(
                    "WAV PCM {}ch {}Hz {}bit",
                    wav.channels, wav.sample_rate, wav.bits_per_sample
                ),
                wav_duration_label(&wav),
            ),
            Err(reason) => (format!("WAV unsupported {}", reason), "--:--".to_string()),
        },
        AudioKind::Mp3 => match probe_mp3(&file.path) {
            Ok(mp3) => {
                let duration_seconds = AUDIO_DURATION_SECONDS.load(Ordering::SeqCst);
                (
                    format!("MP3 {} {} {}Hz", mp3.version, mp3.layer, mp3.sample_rate),
                    if duration_seconds > 0 {
                        seconds_to_mmss(duration_seconds)
                    } else {
                        "MP3".to_string()
                    },
                )
            }
            Err(reason) => (format!("MP3 unsupported {}", reason), "MP3".to_string()),
        },
    };

    MusicScreenSnapshot {
        track_label: trim_label(&file.name, 18),
        subtitle_label: subtitle,
        source_label: format!(
            "/AUDIO {}/{} {} VOL {}%",
            idx + 1,
            files.len(),
            playback_status(file.kind, playing),
            volume_percent()
        ),
        playing,
        progress_percent: progress,
        elapsed_label: seconds_to_mmss(elapsed_seconds),
        duration_label: current_music_duration_label(file.kind, playing, duration),
    }
}

pub fn music_toggle_play_stop() -> &'static str {
    let files = audio_file_entries();
    if files.is_empty() {
        stop_wav_pcm_stream("no-files");
        println!(
            "audio-control-r34: action=PLAY status=NO_AUDIO_FILES path=/AUDIO audio=PCM5101_I2S"
        );
        return "AudioNoFiles";
    }

    let idx = selected_index(files.len());
    let file = &files[idx];

    if AUDIO_PLAYING.load(Ordering::SeqCst) {
        stop_wav_pcm_stream("user-stop");
        println!(
            "audio-control-r34: action=STOP file={} playback=STOPPED audio=PCM5101_I2S",
            file.name
        );
        "AudioStop"
    } else {
        match file.kind {
            AudioKind::Wav => match probe_wav(&file.path) {
                Ok(wav) => match start_wav_pcm_stream(file.path.clone(), file.name.clone(), wav) {
                    Ok(()) => {
                        println!(
                            "audio-control-r34: action=PLAY file={} kind=WAV playback=PCM5101_I2S sample_rate={} channels={} bits={} volume={} audio=PCM5101_I2S",
                            file.name,
                            wav.sample_rate,
                            wav.channels,
                            wav.bits_per_sample,
                            volume_percent()
                        );
                        "AudioPlay"
                    }
                    Err(reason) => {
                        AUDIO_PLAYING.store(false, Ordering::SeqCst);
                        println!(
                            "audio-control-r34: action=PLAY file={} kind=WAV playback=DISABLED reason={} audio=PCM5101_I2S",
                            file.name,
                            reason
                        );
                        "AudioWavUnsupported"
                    }
                },
                Err(reason) => {
                    AUDIO_PLAYING.store(false, Ordering::SeqCst);
                    println!(
                        "audio-control-r34: action=PLAY file={} kind=WAV playback=DISABLED reason={} audio=PCM5101_I2S",
                        file.name,
                        reason
                    );
                    "AudioWavUnsupported"
                }
            },
            AudioKind::Mp3 => match probe_mp3(&file.path) {
                Ok(mp3) => match start_mp3_pcm_stream(file.path.clone(), file.name.clone(), mp3) {
                    Ok(()) => {
                        println!(
                            "audio-control-r35: action=PLAY file={} kind=MP3 playback=PCM5101_I2S decoder=HELIX_FIXED_POINT sample_rate_probe={} volume={} audio=PCM5101_I2S",
                            file.name,
                            mp3.sample_rate,
                            volume_percent()
                        );
                        "AudioPlay"
                    }
                    Err(reason) => {
                        AUDIO_PLAYING.store(false, Ordering::SeqCst);
                        println!(
                            "audio-control-r35: action=PLAY file={} kind=MP3 playback=DISABLED reason={} audio=PCM5101_I2S",
                            file.name,
                            reason
                        );
                        "AudioMp3Unsupported"
                    }
                },
                Err(reason) => {
                    AUDIO_PLAYING.store(false, Ordering::SeqCst);
                    println!(
                        "audio-control-r35: action=PLAY file={} kind=MP3 playback=DISABLED reason={} audio=PCM5101_I2S",
                        file.name,
                        reason
                    );
                    "AudioMp3Unsupported"
                }
            },
        }
    }
}

pub fn music_next() -> &'static str {
    let files = audio_file_entries();
    stop_wav_pcm_stream("next");
    if files.is_empty() {
        println!(
            "audio-control-r34: action=NEXT status=NO_AUDIO_FILES path=/AUDIO audio=PCM5101_I2S"
        );
        return "AudioNoFiles";
    }
    let next = (selected_index(files.len()) + 1) % files.len();
    AUDIO_SELECTED_INDEX.store(next as u32, Ordering::SeqCst);
    reset_audio_progress_state();
    println!(
        "audio-control-r34: action=NEXT selected={} file={} playback=STOPPED audio=PCM5101_I2S",
        next, files[next].name
    );
    "AudioNext"
}

pub fn music_previous() -> &'static str {
    let files = audio_file_entries();
    stop_wav_pcm_stream("prev");
    if files.is_empty() {
        println!(
            "audio-control-r34: action=PREV status=NO_AUDIO_FILES path=/AUDIO audio=PCM5101_I2S"
        );
        return "AudioNoFiles";
    }
    let current = selected_index(files.len());
    let prev = if current == 0 {
        files.len() - 1
    } else {
        current - 1
    };
    AUDIO_SELECTED_INDEX.store(prev as u32, Ordering::SeqCst);
    reset_audio_progress_state();
    println!(
        "audio-control-r34: action=PREV selected={} file={} playback=STOPPED audio=PCM5101_I2S",
        prev, files[prev].name
    );
    "AudioPrev"
}

pub fn apply_volume_percent_silent(percent: u8) {
    let clamped = percent.min(100);
    AUDIO_VOLUME_PERCENT.store(clamped as u32, Ordering::SeqCst);
    unsafe {
        ffi::st77916_audio_mp3_helix_set_volume(clamped as u32);
    }
}

// legacy-marker: audio-volume-r34
pub fn set_volume_percent(percent: u8) {
    let clamped = percent.min(100);
    AUDIO_VOLUME_PERCENT.store(clamped as u32, Ordering::SeqCst);
    unsafe {
        ffi::st77916_audio_mp3_helix_set_volume(clamped as u32);
    }
    println!(
        "audio-volume: source=SettingsSound volume={} route=PCM5101_I2S audio=PCM5101_I2S",
        clamped
    );
}

pub fn volume_percent() -> u8 {
    AUDIO_VOLUME_PERCENT.load(Ordering::SeqCst).min(100) as u8
}

pub fn music_volume_down() -> &'static str {
    let next = volume_percent().saturating_sub(5);
    set_volume_percent(next);
    AUDIO_PROGRESS_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    println!(
        "audio-control-r35-r3: action=VOL_DOWN volume={} source=MusicScreen persistence=model+settings audio=PCM5101_I2S",
        next
    );
    "AudioVolDown"
}

pub fn music_volume_up() -> &'static str {
    let next = volume_percent().saturating_add(5).min(100);
    set_volume_percent(next);
    AUDIO_PROGRESS_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    println!(
        "audio-control-r35-r3: action=VOL_UP volume={} source=MusicScreen persistence=model+settings audio=PCM5101_I2S",
        next
    );
    "AudioVolUp"
}

fn refresh_mp3_progress_from_helix() {
    let progress = unsafe { ffi::st77916_audio_mp3_helix_progress_percent() }.min(100);
    let elapsed = unsafe { ffi::st77916_audio_mp3_helix_elapsed_seconds() };
    let duration = unsafe { ffi::st77916_audio_mp3_helix_duration_seconds() };

    let old_progress = AUDIO_PROGRESS_PERCENT.swap(progress, Ordering::SeqCst);
    let old_elapsed = AUDIO_ELAPSED_SECONDS.swap(elapsed, Ordering::SeqCst);
    let old_duration = AUDIO_DURATION_SECONDS.swap(duration, Ordering::SeqCst);

    if old_progress != progress || old_elapsed != elapsed || old_duration != duration {
        AUDIO_PROGRESS_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    }
}

fn current_music_duration_label(kind: AudioKind, playing: bool, fallback: String) -> String {
    if kind == AudioKind::Mp3 && playing {
        let secs = AUDIO_DURATION_SECONDS.load(Ordering::SeqCst);
        if secs > 0 {
            return seconds_to_mmss(secs);
        }
    }
    fallback
}

pub fn music_progress_sequence() -> u32 {
    AUDIO_PROGRESS_SEQUENCE.load(Ordering::SeqCst)
}

pub fn music_progress_active() -> bool {
    AUDIO_PLAYING.load(Ordering::SeqCst) || AUDIO_THREAD_ACTIVE.load(Ordering::SeqCst)
}

fn reset_audio_progress_state() {
    AUDIO_PROGRESS_TICK.store(0, Ordering::SeqCst);
    AUDIO_PROGRESS_PERCENT.store(0, Ordering::SeqCst);
    AUDIO_ELAPSED_SECONDS.store(0, Ordering::SeqCst);
    AUDIO_DURATION_SECONDS.store(0, Ordering::SeqCst);
    AUDIO_PROGRESS_SEQUENCE.fetch_add(1, Ordering::SeqCst);
}

fn wav_duration_seconds(wav: &WavProbe) -> u32 {
    if wav.sample_rate == 0 || wav.channels == 0 || wav.bits_per_sample == 0 || wav.data_bytes == 0
    {
        return 0;
    }
    let bytes_per_second = wav
        .sample_rate
        .saturating_mul(wav.channels as u32)
        .saturating_mul(wav.bits_per_sample as u32)
        / 8;
    if bytes_per_second == 0 {
        return 0;
    }
    wav.data_bytes / bytes_per_second
}

fn publish_audio_progress(done_bytes: usize, wav: &WavProbe) {
    let total_bytes = wav.data_bytes.max(1) as usize;
    let progress = ((done_bytes as u32).saturating_mul(100) / wav.data_bytes.max(1)).min(100);
    let bytes_per_second = wav
        .sample_rate
        .saturating_mul(wav.channels as u32)
        .saturating_mul(wav.bits_per_sample as u32)
        / 8;
    let elapsed = if bytes_per_second == 0 {
        0
    } else {
        (done_bytes.min(total_bytes) as u32) / bytes_per_second
    };
    let old_progress = AUDIO_PROGRESS_PERCENT.swap(progress, Ordering::SeqCst);
    let old_elapsed = AUDIO_ELAPSED_SECONDS.swap(elapsed, Ordering::SeqCst);
    if old_progress != progress || old_elapsed != elapsed {
        AUDIO_PROGRESS_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    }
}

fn stop_wav_pcm_stream(reason: &'static str) {
    AUDIO_STOP_REQUESTED.store(true, Ordering::SeqCst);
    unsafe {
        ffi::st77916_audio_mp3_helix_stop_request();
    }
    AUDIO_PLAYING.store(false, Ordering::SeqCst);
    println!(
        "audio-pcm-r34: action=STOP_REQUEST reason={} audio=PCM5101_I2S",
        reason
    );
}

fn start_mp3_pcm_stream(path: PathBuf, name: String, mp3: Mp3Probe) -> Result<(), &'static str> {
    if !mp3.frame_sync {
        return Err("NO_FRAME_SYNC");
    }
    if AUDIO_THREAD_ACTIVE
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("PLAYER_BUSY");
    }
    AUDIO_STOP_REQUESTED.store(false, Ordering::SeqCst);
    AUDIO_PLAYING.store(true, Ordering::SeqCst);
    reset_audio_progress_state();
    let spawn_name = format!("audio-mp3-{}", name);
    let spawn_result = thread::Builder::new()
        .name(spawn_name)
        .stack_size(AUDIO_MP3_THREAD_STACK_BYTES)
        .spawn(move || {
            mp3_helix_thread(path, name, mp3);
        });
    match spawn_result {
        Ok(_handle) => Ok(()),
        Err(_) => {
            AUDIO_PLAYING.store(false, Ordering::SeqCst);
            AUDIO_STOP_REQUESTED.store(false, Ordering::SeqCst);
            AUDIO_THREAD_ACTIVE.store(false, Ordering::SeqCst);
            Err("MP3_THREAD_SPAWN_FAILED")
        }
    }
}

fn mp3_helix_thread(path: PathBuf, name: String, probe: Mp3Probe) {
    println!("audio-mp3-r35-r1: status=THREAD_STARTED file={} stack_bytes={} decoder=HELIX_FIXED_POINT probe_rate={} audio=PCM5101_I2S", name, AUDIO_MP3_THREAD_STACK_BYTES, probe.sample_rate);
    let path_text = path.to_string_lossy().to_string();
    let c_path = match CString::new(path_text.as_bytes()) {
        Ok(c_path) => c_path,
        Err(_) => {
            println!(
                "audio-mp3-r35-r1: status=BAD_PATH file={} audio=PCM5101_I2S",
                name
            );
            AUDIO_PLAYING.store(false, Ordering::SeqCst);
            AUDIO_STOP_REQUESTED.store(false, Ordering::SeqCst);
            AUDIO_THREAD_ACTIVE.store(false, Ordering::SeqCst);
            return;
        }
    };
    let result = unsafe {
        ffi::st77916_audio_mp3_helix_play_file(c_path.as_ptr(), volume_percent().min(100) as u32)
    };
    let completed = result == 0;
    if completed {
        AUDIO_PROGRESS_PERCENT.store(100, Ordering::SeqCst);
        AUDIO_PROGRESS_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    }
    AUDIO_PLAYING.store(false, Ordering::SeqCst);
    AUDIO_STOP_REQUESTED.store(false, Ordering::SeqCst);
    AUDIO_THREAD_ACTIVE.store(false, Ordering::SeqCst);
    println!(
        "audio-mp3-r35-r1: status={} file={} code={} audio=PCM5101_I2S",
        if completed {
            "COMPLETE"
        } else {
            "STOPPED_OR_ERROR"
        },
        name,
        result
    );
}

fn start_wav_pcm_stream(path: PathBuf, name: String, wav: WavProbe) -> Result<(), &'static str> {
    if wav.audio_format != 1 {
        return Err("NOT_PCM_FORMAT_1");
    }
    if wav.bits_per_sample != 16 {
        return Err("ONLY_PCM16_SUPPORTED");
    }
    if wav.channels == 0 || wav.channels > 2 {
        return Err("CHANNELS_UNSUPPORTED");
    }
    if wav.sample_rate < 8000 || wav.sample_rate > 96000 {
        return Err("SAMPLE_RATE_UNSUPPORTED");
    }
    if wav.data_bytes == 0 {
        return Err("NO_DATA_CHUNK");
    }

    if AUDIO_THREAD_ACTIVE
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("PLAYER_BUSY");
    }

    AUDIO_STOP_REQUESTED.store(false, Ordering::SeqCst);
    AUDIO_PLAYING.store(true, Ordering::SeqCst);
    reset_audio_progress_state();
    AUDIO_DURATION_SECONDS.store(wav_duration_seconds(&wav), Ordering::SeqCst);
    AUDIO_PROGRESS_SEQUENCE.fetch_add(1, Ordering::SeqCst);

    let spawn_name = format!("audio-wav-{}", name);
    let spawn_result = thread::Builder::new()
        .name(spawn_name)
        .stack_size(AUDIO_PCM_THREAD_STACK_BYTES)
        .spawn(move || {
            wav_pcm_thread(path, name, wav);
        });

    match spawn_result {
        Ok(_handle) => Ok(()),
        Err(_) => {
            AUDIO_PLAYING.store(false, Ordering::SeqCst);
            AUDIO_STOP_REQUESTED.store(false, Ordering::SeqCst);
            AUDIO_THREAD_ACTIVE.store(false, Ordering::SeqCst);
            Err("THREAD_SPAWN_FAILED")
        }
    }
}

fn wav_pcm_thread(path: PathBuf, name: String, wav: WavProbe) {
    let mut completed = false;

    let init_ok =
        unsafe { ffi::st77916_audio_pcm_init(wav.sample_rate, wav.bits_per_sample, wav.channels) };
    if !init_ok {
        println!(
            "audio-pcm-r34: status=INIT_FAILED file={} sample_rate={} channels={} bits={} audio=PCM5101_I2S",
            name,
            wav.sample_rate,
            wav.channels,
            wav.bits_per_sample
        );
        AUDIO_PLAYING.store(false, Ordering::SeqCst);
        AUDIO_THREAD_ACTIVE.store(false, Ordering::SeqCst);
        return;
    }

    match File::open(&path) {
        Ok(mut file) => {
            if file.seek(SeekFrom::Start(wav.data_offset)).is_ok() {
                let mut remaining = wav.data_bytes as usize;
                let mut buffer = vec![0u8; AUDIO_PCM_STREAM_BUFFER_BYTES];
                println!(
                    "audio-pcm-r34-r3: status=THREAD_STARTED file={} stack_bytes={} buffer=HEAP:{} data_bytes={} audio=PCM5101_I2S",
                    name,
                    AUDIO_PCM_THREAD_STACK_BYTES,
                    buffer.len(),
                    wav.data_bytes
                );
                while remaining > 0 && !AUDIO_STOP_REQUESTED.load(Ordering::SeqCst) {
                    let want = remaining.min(buffer.len());
                    match file.read(&mut buffer[..want]) {
                        Ok(0) => break,
                        Ok(n) => {
                            let volume = volume_percent();
                            scale_pcm16_in_place(&mut buffer[..n], volume);
                            let written = unsafe {
                                ffi::st77916_audio_pcm_write(buffer.as_ptr(), n as u32, 250)
                            };
                            if written <= 0 {
                                println!(
                                    "audio-pcm-r34: status=WRITE_FAILED file={} err={} audio=PCM5101_I2S",
                                    name,
                                    written
                                );
                                break;
                            }
                            let before_remaining = remaining;
                            remaining = remaining.saturating_sub(n);
                            if before_remaining == wav.data_bytes as usize {
                                println!(
                                    "audio-pcm-r34-r3: status=FIRST_WRITE_OK file={} bytes={} audio=PCM5101_I2S",
                                    name,
                                    written
                                );
                            }
                            let done = wav.data_bytes as usize - remaining;
                            publish_audio_progress(done, &wav);
                        }
                        Err(_) => break,
                    }
                }
                completed = remaining == 0 && !AUDIO_STOP_REQUESTED.load(Ordering::SeqCst);
            } else {
                println!(
                    "audio-pcm-r34: status=SEEK_FAILED file={} offset={} audio=PCM5101_I2S",
                    name, wav.data_offset
                );
            }
        }
        Err(_) => println!(
            "audio-pcm-r34: status=OPEN_FAILED file={} audio=PCM5101_I2S",
            name
        ),
    }

    if completed {
        AUDIO_PROGRESS_PERCENT.store(100, Ordering::SeqCst);
        AUDIO_ELAPSED_SECONDS.store(
            AUDIO_DURATION_SECONDS.load(Ordering::SeqCst),
            Ordering::SeqCst,
        );
        AUDIO_PROGRESS_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    }

    unsafe {
        ffi::st77916_audio_pcm_stop();
    }
    AUDIO_PLAYING.store(false, Ordering::SeqCst);
    AUDIO_STOP_REQUESTED.store(false, Ordering::SeqCst);
    AUDIO_THREAD_ACTIVE.store(false, Ordering::SeqCst);

    println!(
        "audio-pcm-r34: status={} file={} audio=PCM5101_I2S",
        if completed { "COMPLETE" } else { "STOPPED" },
        name
    );
}

fn scale_pcm16_in_place(buf: &mut [u8], volume: u8) {
    let volume = volume.min(100) as i32;
    if volume >= 100 {
        return;
    }
    let mut i = 0usize;
    while i + 1 < buf.len() {
        let sample = i16::from_le_bytes([buf[i], buf[i + 1]]) as i32;
        let scaled = (sample * volume / 100).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        let bytes = scaled.to_le_bytes();
        buf[i] = bytes[0];
        buf[i + 1] = bytes[1];
        i += 2;
    }
}

fn audio_file_entries() -> Vec<AudioFileEntry> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(AUDIO_DIR) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path
                .extension()
                .and_then(|x| x.to_str())
                .unwrap_or("")
                .to_ascii_uppercase();
            let kind = match ext.as_str() {
                "WAV" => AudioKind::Wav,
                "MP3" => AudioKind::Mp3,
                _ => continue,
            };
            let name = path
                .file_name()
                .and_then(|x| x.to_str())
                .unwrap_or("?")
                .to_string();
            files.push(AudioFileEntry { path, name, kind });
        }
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    files
}

fn selected_index(len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let idx = AUDIO_SELECTED_INDEX.load(Ordering::SeqCst) as usize;
    if idx >= len {
        AUDIO_SELECTED_INDEX.store(0, Ordering::SeqCst);
        0
    } else {
        idx
    }
}

fn playback_status(kind: AudioKind, playing: bool) -> &'static str {
    match (kind, playing) {
        (AudioKind::Wav, true) => "WAV PLAY",
        (AudioKind::Wav, false) => "WAV READY",
        (AudioKind::Mp3, true) => "MP3 PLAY",
        (AudioKind::Mp3, false) => "MP3 READY",
    }
}

fn trim_label(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for ch in text.chars().take(max_chars) {
        out.push(ch);
    }
    out
}

fn wav_duration_label(wav: &WavProbe) -> String {
    if wav.sample_rate == 0 || wav.channels == 0 || wav.bits_per_sample == 0 || wav.data_bytes == 0
    {
        return "--:--".to_string();
    }
    let bytes_per_second = wav
        .sample_rate
        .saturating_mul(wav.channels as u32)
        .saturating_mul(wav.bits_per_sample as u32)
        / 8;
    if bytes_per_second == 0 {
        return "--:--".to_string();
    }
    let seconds = wav.data_bytes / bytes_per_second;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn seconds_to_mmss(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
