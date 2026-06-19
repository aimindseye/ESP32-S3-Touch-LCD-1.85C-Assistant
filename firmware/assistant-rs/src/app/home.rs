#[derive(Debug, Clone)]
pub struct HomeState {
    pub status_detail_open: bool,
    pub select_count: u32,
}

impl HomeState {
    pub const fn new() -> Self {
        Self {
            status_detail_open: false,
            select_count: 0,
        }
    }

    pub fn toggle_status_detail(&mut self) {
        self.status_detail_open = !self.status_detail_open;
        self.select_count = self.select_count.saturating_add(1);
    }

    pub const fn detail_label(&self) -> &'static str {
        if self.status_detail_open {
            "DETAIL ON"
        } else {
            "DETAIL OFF"
        }
    }
}
