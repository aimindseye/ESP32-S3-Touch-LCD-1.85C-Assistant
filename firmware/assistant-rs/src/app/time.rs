//! Time sync and RTC persistence app state.
//!
//! v0.1.19 uses the hardware RTC immediately at boot, then starts SNTP after
//! Wi-Fi is connected. Once SNTP completes, local time is written back to the
//! PCF85063 RTC.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSource {
    Unsynced,
    Rtc,
    Ntp,
}

impl TimeSource {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unsynced => "UNSYNCED",
            Self::Rtc => "RTC",
            Self::Ntp => "NTP",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSyncPhase {
    Idle,
    Waiting,
    Synced,
    Persisted,
    Failed,
}

impl TimeSyncPhase {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Idle => "IDLE",
            Self::Waiting => "WAIT",
            Self::Synced => "SYNCED",
            Self::Persisted => "RTC SAVE",
            Self::Failed => "FAILED",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimeSyncState {
    pub source: TimeSource,
    pub phase: TimeSyncPhase,
    pub attempts: u32,
    pub rtc_boot_read: bool,
    pub rtc_persisted: bool,
    pub last_error: &'static str,
}

impl TimeSyncState {
    pub const fn new() -> Self {
        Self {
            source: TimeSource::Unsynced,
            phase: TimeSyncPhase::Idle,
            attempts: 0,
            rtc_boot_read: false,
            rtc_persisted: false,
            last_error: "NONE",
        }
    }

    pub fn note_rtc_boot_read(&mut self) {
        self.source = TimeSource::Rtc;
        self.rtc_boot_read = true;
        self.last_error = "NONE";
    }

    pub fn note_rtc_boot_failed(&mut self) {
        self.source = TimeSource::Unsynced;
        self.last_error = "RTC READ";
    }

    pub fn start_sntp_wait(&mut self) {
        self.phase = TimeSyncPhase::Waiting;
        self.attempts = self.attempts.saturating_add(1);
        self.last_error = "SNTP";
    }

    pub fn note_sntp_synced(&mut self) {
        self.source = TimeSource::Ntp;
        self.phase = TimeSyncPhase::Synced;
        self.last_error = "NONE";
    }

    pub fn note_rtc_persisted(&mut self) {
        self.source = TimeSource::Ntp;
        self.phase = TimeSyncPhase::Persisted;
        self.rtc_persisted = true;
        self.last_error = "NONE";
    }

    pub fn note_failed(&mut self, reason: &'static str) {
        self.phase = TimeSyncPhase::Failed;
        self.last_error = reason;
    }

    pub const fn should_start_after_wifi(&self) -> bool {
        !self.rtc_persisted
            && !matches!(
                self.phase,
                TimeSyncPhase::Waiting | TimeSyncPhase::Persisted
            )
    }

    pub const fn status_label(&self) -> &'static str {
        self.phase.label()
    }

    pub const fn source_label(&self) -> &'static str {
        self.source.label()
    }
}

pub const TIME_SYNC_RTC_MARKER: &str =
    "v0.1.19 time sync rtc persistence foundation: rtc boot plus sntp writeback";
