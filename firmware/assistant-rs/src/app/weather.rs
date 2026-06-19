#[derive(Debug, Clone)]
pub struct WeatherState {
    pub refresh_attempts: u32,
    pub status: &'static str,
    pub temperature_label: &'static str,
    pub condition_label: &'static str,
}

impl WeatherState {
    pub const fn new() -> Self {
        Self {
            refresh_attempts: 0,
            status: "MOCK READY",
            temperature_label: "--",
            condition_label: "LOCAL ONLY",
        }
    }

    pub fn refresh_mock(&mut self) {
        self.refresh_attempts = self.refresh_attempts.saturating_add(1);

        match self.refresh_attempts % 4 {
            1 => {
                self.status = "MOCK SAMPLE 1";
                self.temperature_label = "72F";
                self.condition_label = "CLEAR MOCK";
            }
            2 => {
                self.status = "MOCK SAMPLE 2";
                self.temperature_label = "68F";
                self.condition_label = "CLOUDY MOCK";
            }
            3 => {
                self.status = "MOCK SAMPLE 3";
                self.temperature_label = "75F";
                self.condition_label = "SUNNY MOCK";
            }
            _ => {
                self.status = "MOCK SAMPLE 4";
                self.temperature_label = "70F";
                self.condition_label = "BREEZY MOCK";
            }
        }
    }
}
