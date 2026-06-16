unsafe extern "C" {
    pub fn st77916_panel_init() -> bool;
    pub fn st77916_panel_draw_rgb565(
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
        color: *mut u16,
    ) -> bool;
    pub fn st77916_probe_sd_capacity_mb(
        out_present: *mut bool,
        out_capacity_mb: *mut u32,
    ) -> bool;
}