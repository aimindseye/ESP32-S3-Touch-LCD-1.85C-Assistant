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

    pub fn refresh_glance(&mut self) {
        // v0.1.22: Home is glance-only. Center tap requests a data refresh
        // and does not open a Home detail mode.
        self.status_detail_open = false;
        self.select_count = self.select_count.saturating_add(1);
    }

    pub const fn detail_label(&self) -> &'static str {
        "GLANCE"
    }
}
