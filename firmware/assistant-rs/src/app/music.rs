#[derive(Debug, Clone)]
pub struct MusicState {
    pub playing: bool,
    pub toggle_count: u32,
    pub source: &'static str,
    pub track_label: &'static str,
    pub subtitle_label: &'static str,
}

impl MusicState {
    pub const fn new() -> Self {
        Self {
            playing: false,
            toggle_count: 0,
            source: "LOCAL MOCK",
            track_label: "MIDNIGHT",
            subtitle_label: "LOCAL MOCK",
        }
    }

    pub fn toggle_play_pause(&mut self) {
        self.playing = !self.playing;
        self.toggle_count = self.toggle_count.saturating_add(1);

        match self.toggle_count % 3 {
            1 => {
                self.track_label = "MIDNIGHT";
                self.subtitle_label = "CHILLWAVE";
            }
            2 => {
                self.track_label = "NEON DRIVE";
                self.subtitle_label = "LOCAL MOCK";
            }
            _ => {
                self.track_label = "LOFI BEATS";
                self.subtitle_label = "LOCAL MOCK";
            }
        }
    }

    pub const fn state_label(&self) -> &'static str {
        if self.playing {
            "PLAYING"
        } else {
            "PAUSED"
        }
    }

    pub const fn elapsed_label(&self) -> &'static str {
        match self.toggle_count % 3 {
            1 => "1:28",
            2 => "0:42",
            _ => "2:05",
        }
    }

    pub const fn duration_label(&self) -> &'static str {
        match self.toggle_count % 3 {
            1 => "3:45",
            2 => "2:58",
            _ => "4:10",
        }
    }

    pub const fn progress_percent(&self) -> u8 {
        match self.toggle_count % 3 {
            1 => 38,
            2 => 24,
            _ => 55,
        }
    }
}
