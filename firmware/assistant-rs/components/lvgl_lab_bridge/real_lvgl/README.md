# Real LVGL experiment notes

The default v0.1.8 package compiles a safe bridge component and renders a single LVGL-lab style test frame through `lvgl_lab_render_test_rgb565()`.

For a future real LVGL experiment:
1. Copy `idf_component.yml.example` to `components/lvgl_lab_bridge/idf_component.yml`.
2. Add real LVGL/esp_lvgl_port initialization code behind a compile-time flag.
3. Keep CST816/r12 touch outside LVGL until visual quality is proven.
4. Keep the raw renderer and RGB565 asset renderer as fallbacks.

This separation prevents the first lab step from breaking the accepted renderer baseline.
