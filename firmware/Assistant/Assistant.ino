#include <Arduino.h>
#include <stdint.h>

#include "Display_ST77916.h"
#include "I2C_Driver.h"
#include "TCA9554PWR.h"
#include "BAT_Driver.h"
#include "RTC_PCF85063.h"
#include "SD_Card.h"
#include "LVGL_Driver.h"

#include "FS.h"
#include "SD_MMC.h"
#include "Audio.h"

// Set to 1 only once if you want to initialize the RTC.
// After a successful flash and time set, change it back to 0 and reflash.
#define RTC_SET_TIME_ON_BOOT 0

// V1 board audio playback path (PCM5101)
static constexpr int I2S_BCLK = 48;
static constexpr int I2S_LRC  = 38;
static constexpr int I2S_DOUT = 47;

static const char *TRACK_TITLES[] = {
  "Bollywood Track 1",
  "Bollywood Track 2",
  "Bollywood Track 3"
};

static const char *TRACK_PATHS[] = {
  "/solarflex-bollywood-indian-hindi-song-509913.mp3",
  "/bounce-bay-records-bollywood-1-437912.mp3",
  "/freemusicforvideo-bollywood-indian-hindi-song-music-504893.mp3"
};

static constexpr size_t TRACK_COUNT = sizeof(TRACK_PATHS) / sizeof(TRACK_PATHS[0]);

Audio audio;

// -------------------------------
// UI state
// -------------------------------
static lv_obj_t *g_status_label = nullptr;
static lv_obj_t *g_battery_label = nullptr;
static lv_obj_t *g_backlight_label = nullptr;
static lv_obj_t *g_counter_label = nullptr;
static lv_obj_t *g_time_label = nullptr;
static lv_obj_t *g_sd_label = nullptr;

static lv_obj_t *g_audio_title_label = nullptr;
static lv_obj_t *g_audio_subtitle_label = nullptr;
static lv_obj_t *g_audio_time_label = nullptr;
static lv_obj_t *g_volume_label = nullptr;
static lv_obj_t *g_cover_card = nullptr;
static lv_obj_t *g_cover_badge_label = nullptr;
static lv_obj_t *g_cover_caption_label = nullptr;
static lv_obj_t *g_play_btn_label = nullptr;

static lv_obj_t *g_home_page = nullptr;
static lv_obj_t *g_audio_page = nullptr;
static lv_obj_t *g_home_tab_btn = nullptr;
static lv_obj_t *g_audio_tab_btn = nullptr;

static uint32_t g_touch_count = 0;
static int g_audio_volume = 12;
static int g_current_track = -1;
static bool g_sd_ok = false;
static bool g_audio_page_active = false;
static bool g_audio_paused = false;
static bool g_track_art_scanned[TRACK_COUNT] = {false, false, false};
static bool g_track_has_embedded_art[TRACK_COUNT] = {false, false, false};

static lv_color_t kBgColor        = lv_color_hex(0x101418);
static lv_color_t kTextColor      = lv_color_hex(0xE2E8F0);
static lv_color_t kMutedTextColor = lv_color_hex(0x9AA4B2);
static lv_color_t kActiveTabColor = lv_color_hex(0x334155);
static lv_color_t kIdleTabColor   = lv_color_hex(0x1E293B);
static lv_color_t kCardColor      = lv_color_hex(0x17212B);
static lv_color_t kAccentColor    = lv_color_hex(0x2563EB);
static lv_color_t kArtColor       = lv_color_hex(0x0F766E);

// -------------------------------
// Helpers
// -------------------------------
static lv_obj_t *make_row_label(lv_obj_t *parent, const char *text, lv_color_t color, lv_text_align_t align = LV_TEXT_ALIGN_LEFT) {
  lv_obj_t *lbl = lv_label_create(parent);
  lv_label_set_text(lbl, text);
  lv_obj_set_width(lbl, 188);
  lv_label_set_long_mode(lbl, LV_LABEL_LONG_WRAP);
  lv_obj_set_style_text_align(lbl, align, 0);
  lv_obj_set_style_text_color(lbl, color, 0);
  return lbl;
}

static lv_obj_t *make_button(lv_obj_t *parent, int w, int h, const char *text) {
  lv_obj_t *btn = lv_button_create(parent);
  lv_obj_set_size(btn, w, h);
  lv_obj_set_style_radius(btn, 16, 0);

  lv_obj_t *lbl = lv_label_create(btn);
  lv_label_set_text(lbl, text);
  lv_obj_center(lbl);
  return btn;
}

static uint32_t read_synchsafe_u32(const uint8_t *p) {
  return ((uint32_t)(p[0] & 0x7F) << 21) |
         ((uint32_t)(p[1] & 0x7F) << 14) |
         ((uint32_t)(p[2] & 0x7F) << 7)  |
         ((uint32_t)(p[3] & 0x7F));
}

static uint32_t read_be_u32(const uint8_t *p) {
  return ((uint32_t)p[0] << 24) |
         ((uint32_t)p[1] << 16) |
         ((uint32_t)p[2] << 8)  |
         ((uint32_t)p[3]);
}

static void format_mmss(uint32_t seconds, char *buf, size_t buf_len) {
  uint32_t mm = seconds / 60U;
  uint32_t ss = seconds % 60U;
  snprintf(buf, buf_len, "%02u:%02u", (unsigned)mm, (unsigned)ss);
}

static bool mp3_has_embedded_art(const char *path) {
  File f = SD_MMC.open(path);
  if (!f) return false;

  if (f.size() < 10) {
    f.close();
    return false;
  }

  uint8_t hdr[10];
  if (f.read(hdr, sizeof(hdr)) != (int)sizeof(hdr)) {
    f.close();
    return false;
  }

  if (!(hdr[0] == 'I' && hdr[1] == 'D' && hdr[2] == '3')) {
    f.close();
    return false;
  }

  const uint8_t version = hdr[3];
  const uint32_t tag_size = read_synchsafe_u32(&hdr[6]);

  uint32_t pos = 10;
  while (pos + 10 <= (10 + tag_size) && pos + 10 <= (uint32_t)f.size()) {
    uint8_t frame_hdr[10];
    f.seek(pos);
    if (f.read(frame_hdr, sizeof(frame_hdr)) != (int)sizeof(frame_hdr)) break;

    bool empty = true;
    for (uint8_t i = 0; i < 10; ++i) {
      if (frame_hdr[i] != 0) {
        empty = false;
        break;
      }
    }
    if (empty) break;

    char frame_id[5] = {0};
    frame_id[0] = (char)frame_hdr[0];
    frame_id[1] = (char)frame_hdr[1];
    frame_id[2] = (char)frame_hdr[2];
    frame_id[3] = (char)frame_hdr[3];

    uint32_t frame_size = 0;
    if (version == 4) frame_size = read_synchsafe_u32(&frame_hdr[4]);
    else frame_size = read_be_u32(&frame_hdr[4]);

    if (frame_size == 0) break;

    if (strcmp(frame_id, "APIC") == 0) {
      f.close();
      return true;
    }

    pos += 10 + frame_size;
  }

  f.close();
  return false;
}

static void ensure_track_art_scanned(size_t idx) {
  if (idx >= TRACK_COUNT) return;
  if (g_track_art_scanned[idx]) return;

  g_track_has_embedded_art[idx] = mp3_has_embedded_art(TRACK_PATHS[idx]);
  g_track_art_scanned[idx] = true;
}

static void update_tab_styles() {
  if (g_home_tab_btn == nullptr || g_audio_tab_btn == nullptr) return;

  lv_obj_set_style_bg_color(
    g_home_tab_btn,
    g_audio_page_active ? kIdleTabColor : kActiveTabColor,
    0
  );
  lv_obj_set_style_bg_color(
    g_audio_tab_btn,
    g_audio_page_active ? kActiveTabColor : kIdleTabColor,
    0
  );
}

static void show_audio_page(bool show_audio) {
  g_audio_page_active = show_audio;

  if (g_home_page) {
    if (show_audio) lv_obj_add_flag(g_home_page, LV_OBJ_FLAG_HIDDEN);
    else lv_obj_remove_flag(g_home_page, LV_OBJ_FLAG_HIDDEN);
  }

  if (g_audio_page) {
    if (show_audio) lv_obj_remove_flag(g_audio_page, LV_OBJ_FLAG_HIDDEN);
    else lv_obj_add_flag(g_audio_page, LV_OBJ_FLAG_HIDDEN);
  }

  update_tab_styles();
}

static void update_backlight_label(uint8_t value) {
  if (g_backlight_label == nullptr) return;
  lv_label_set_text_fmt(g_backlight_label, "Light: %u%%", value);
}

static void update_volume_label() {
  if (g_volume_label == nullptr) return;
  lv_label_set_text_fmt(g_volume_label, "Vol: %d", g_audio_volume);
}

static void update_cover_card() {
  if (!g_cover_card || !g_cover_badge_label || !g_cover_caption_label) return;

  if (g_current_track >= 0 && g_current_track < (int)TRACK_COUNT) {
    ensure_track_art_scanned((size_t)g_current_track);

    if (g_track_has_embedded_art[g_current_track]) {
      lv_obj_set_style_bg_color(g_cover_card, kArtColor, 0);
      lv_label_set_text(g_cover_badge_label, "ART");
      lv_label_set_text(g_cover_caption_label, "Embedded cover found");
    } else {
      lv_obj_set_style_bg_color(g_cover_card, kCardColor, 0);
      lv_label_set_text(g_cover_badge_label, "MP3");
      lv_label_set_text(g_cover_caption_label, "No embedded cover");
    }
  } else {
    lv_obj_set_style_bg_color(g_cover_card, kCardColor, 0);
    lv_label_set_text(g_cover_badge_label, "MP3");
    lv_label_set_text(g_cover_caption_label, "Select a track");
  }
}

static void update_audio_title() {
  if (g_audio_title_label == nullptr || g_audio_subtitle_label == nullptr) return;

  if (g_current_track < 0 || g_current_track >= (int)TRACK_COUNT) {
    lv_label_set_text(g_audio_title_label, "Audio idle");
    lv_label_set_text(g_audio_subtitle_label, "Ready to play");
    return;
  }

  lv_label_set_text(g_audio_title_label, TRACK_TITLES[g_current_track]);

  if (g_audio_paused) {
    lv_label_set_text(g_audio_subtitle_label, "Paused");
  } else if (audio.isRunning()) {
    lv_label_set_text(g_audio_subtitle_label, "Playing");
  } else {
    lv_label_set_text(g_audio_subtitle_label, "Stopped");
  }
}

static void update_audio_time_label() {
  if (g_audio_time_label == nullptr) return;

  if (g_current_track < 0) {
    lv_label_set_text(g_audio_time_label, "00:00 / 00:00");
    return;
  }

  char now_buf[8];
  char dur_buf[8];
  format_mmss(audio.getAudioCurrentTime(), now_buf, sizeof(now_buf));
  format_mmss(audio.getAudioFileDuration(), dur_buf, sizeof(dur_buf));
  lv_label_set_text_fmt(g_audio_time_label, "%s / %s", now_buf, dur_buf);
}

static void update_play_button_icon() {
  if (g_play_btn_label == nullptr) return;

  if (audio.isRunning() && !g_audio_paused) {
    lv_label_set_text(g_play_btn_label, LV_SYMBOL_PAUSE);
  } else {
    lv_label_set_text(g_play_btn_label, LV_SYMBOL_PLAY);
  }
}

static void update_audio_ui() {
  update_audio_title();
  update_audio_time_label();
  update_cover_card();
  update_play_button_icon();
}

static void update_time_label() {
  if (g_time_label == nullptr) return;

  PCF85063_Loop();

  if (datetime.year < 2024 || datetime.year > 2099 ||
      datetime.month == 0 || datetime.month > 12 ||
      datetime.day == 0 || datetime.day > 31) {
    lv_label_set_text(g_time_label, "RTC: not set");
    return;
  }

  lv_label_set_text_fmt(
    g_time_label,
    "RTC: %02u:%02u:%02u",
    (unsigned)datetime.hour,
    (unsigned)datetime.minute,
    (unsigned)datetime.second
  );
}

static void update_sd_label() {
  if (g_sd_label == nullptr) return;

  if (!g_sd_ok || SD_MMC.cardType() == CARD_NONE) {
    lv_label_set_text(g_sd_label, "SD: not detected");
    return;
  }

  uint64_t totalBytes = SD_MMC.totalBytes();
  uint64_t usedBytes = SD_MMC.usedBytes();

  lv_label_set_text_fmt(
    g_sd_label,
    "SD: %u/%uM",
    (unsigned)(usedBytes / (1024ULL * 1024ULL)),
    (unsigned)(totalBytes / (1024ULL * 1024ULL))
  );
}

static void ui_refresh_timer_cb(lv_timer_t *timer) {
  LV_UNUSED(timer);

  if (g_status_label) {
    lv_label_set_text_fmt(
      g_status_label,
      "Heap: %u KB",
      (unsigned)(ESP.getFreeHeap() / 1024)
    );
  }

  if (g_battery_label) {
    char buf[32];
    const float volts = BAT_Get_Volts();
    snprintf(buf, sizeof(buf), "Batt: %.2f V", volts);
    lv_label_set_text(g_battery_label, buf);
  }

  update_time_label();
  update_sd_label();
  update_audio_ui();
  update_volume_label();
}

static bool play_track_index(size_t idx) {
  if (!g_sd_ok) {
    Serial.println("Audio: SD not ready");
    return false;
  }

  if (idx >= TRACK_COUNT) {
    Serial.println("Audio: invalid track index");
    return false;
  }

  audio.stopSong();
  delay(20);

  Serial.printf("Audio: starting %s\n", TRACK_PATHS[idx]);

  if (!audio.connecttoFS(SD_MMC, TRACK_PATHS[idx])) {
    Serial.printf("Audio: failed to start %s\n", TRACK_PATHS[idx]);
    return false;
  }

  g_current_track = (int)idx;
  g_audio_paused = false;
  ensure_track_art_scanned(idx);
  update_audio_ui();
  return true;
}

static void play_or_resume_current() {
  if (g_current_track < 0) {
    play_track_index(0);
    return;
  }

  if (!audio.isRunning() && !g_audio_paused) {
    play_track_index((size_t)g_current_track);
    return;
  }

  if (audio.pauseResume()) {
    g_audio_paused = !g_audio_paused;
  } else if (!audio.isRunning()) {
    play_track_index((size_t)g_current_track);
  }

  update_audio_ui();
}

static void stop_current_track() {
  audio.stopSong();
  g_audio_paused = false;
  update_audio_ui();
}

static void backlight_slider_event_cb(lv_event_t *e) {
  lv_obj_t *slider = static_cast<lv_obj_t *>(lv_event_get_target(e));
  const int32_t value = lv_slider_get_value(slider);

  Set_Backlight((uint8_t)value);
  update_backlight_label((uint8_t)value);
}

static void volume_slider_event_cb(lv_event_t *e) {
  lv_obj_t *slider = static_cast<lv_obj_t *>(lv_event_get_target(e));
  g_audio_volume = (int)lv_slider_get_value(slider);
  audio.setVolume(g_audio_volume);
  update_volume_label();
}

static void touch_test_button_event_cb(lv_event_t *e) {
  LV_UNUSED(e);
  g_touch_count++;

  if (g_counter_label) {
    lv_label_set_text_fmt(
      g_counter_label,
      "Touch count: %" LV_PRIu32,
      g_touch_count
    );
  }
}

static void prev_track_event_cb(lv_event_t *e) {
  LV_UNUSED(e);

  size_t idx = 0;
  if (g_current_track < 0) idx = TRACK_COUNT - 1;
  else idx = (size_t)((g_current_track + (int)TRACK_COUNT - 1) % (int)TRACK_COUNT);

  play_track_index(idx);
}

static void play_pause_event_cb(lv_event_t *e) {
  LV_UNUSED(e);
  play_or_resume_current();
}

static void next_track_event_cb(lv_event_t *e) {
  LV_UNUSED(e);

  size_t idx = 0;
  if (g_current_track < 0) idx = 0;
  else idx = (size_t)((g_current_track + 1) % (int)TRACK_COUNT);

  play_track_index(idx);
}

static void stop_track_event_cb(lv_event_t *e) {
  LV_UNUSED(e);
  stop_current_track();
}

static void home_tab_event_cb(lv_event_t *e) {
  LV_UNUSED(e);
  show_audio_page(false);
}

static void audio_tab_event_cb(lv_event_t *e) {
  LV_UNUSED(e);
  show_audio_page(true);
}

static void print_root_listing() {
  File root = SD_MMC.open("/");
  if (!root || !root.isDirectory()) {
    Serial.println("SD: failed to open root");
    return;
  }

  Serial.println("SD root listing:");
  bool any = false;

  File file = root.openNextFile();
  while (file) {
    any = true;
    Serial.printf(
      "  %s%s (%u bytes)\n",
      file.isDirectory() ? "[DIR] " : "",
      file.name(),
      (unsigned)file.size()
    );
    file = root.openNextFile();
  }

  if (!any) {
    Serial.println("  <empty>");
  }
}

static void build_home_page(lv_obj_t *parent) {
  g_home_page = lv_obj_create(parent);
  lv_obj_remove_style_all(g_home_page);
  lv_obj_set_size(g_home_page, 196, 188);
  lv_obj_set_pos(g_home_page, 0, 0);
  lv_obj_set_style_bg_opa(g_home_page, LV_OPA_TRANSP, 0);
  lv_obj_set_layout(g_home_page, LV_LAYOUT_FLEX);
  lv_obj_set_flex_flow(g_home_page, LV_FLEX_FLOW_COLUMN);
  lv_obj_set_flex_align(g_home_page, LV_FLEX_ALIGN_START, LV_FLEX_ALIGN_CENTER, LV_FLEX_ALIGN_CENTER);
  lv_obj_set_style_pad_all(g_home_page, 0, 0);
  lv_obj_set_style_pad_row(g_home_page, 6, 0);

  g_status_label = make_row_label(g_home_page, "Heap: -- KB", kTextColor);
  g_battery_label = make_row_label(g_home_page, "Batt: --.-- V", kTextColor);
  g_time_label = make_row_label(g_home_page, "RTC: --:--:--", kTextColor);
  g_sd_label = make_row_label(g_home_page, "SD: --", kTextColor);
  g_backlight_label = make_row_label(g_home_page, "Light: 50%", kTextColor);

  lv_obj_t *backlight_slider = lv_slider_create(g_home_page);
  lv_obj_set_size(backlight_slider, 164, 12);
  lv_slider_set_range(backlight_slider, 5, 100);
  lv_slider_set_value(backlight_slider, LCD_Backlight, LV_ANIM_OFF);
  lv_obj_add_event_cb(backlight_slider, backlight_slider_event_cb, LV_EVENT_VALUE_CHANGED, nullptr);

  g_counter_label = make_row_label(g_home_page, "Touch count: 0", kTextColor, LV_TEXT_ALIGN_CENTER);
  lv_obj_set_style_text_align(g_counter_label, LV_TEXT_ALIGN_CENTER, 0);

  lv_obj_t *touch_btn = make_button(g_home_page, 146, 30, "Touch test");
  lv_obj_add_event_cb(touch_btn, touch_test_button_event_cb, LV_EVENT_CLICKED, nullptr);
}

static void build_audio_page(lv_obj_t *parent) {
  g_audio_page = lv_obj_create(parent);
  lv_obj_remove_style_all(g_audio_page);
  lv_obj_set_size(g_audio_page, 196, 248);
  lv_obj_set_pos(g_audio_page, 0, 0);
  lv_obj_set_style_bg_opa(g_audio_page, LV_OPA_TRANSP, 0);

  // Cover card
  g_cover_card = lv_obj_create(g_audio_page);
  lv_obj_set_size(g_cover_card, 112, 112);
  lv_obj_set_pos(g_cover_card, 42, 0);
  lv_obj_set_style_radius(g_cover_card, 22, 0);
  lv_obj_set_style_bg_color(g_cover_card, kCardColor, 0);
  lv_obj_set_style_border_width(g_cover_card, 1, 0);
  lv_obj_set_style_border_color(g_cover_card, lv_color_hex(0x2B3A49), 0);

  g_cover_badge_label = lv_label_create(g_cover_card);
  lv_label_set_text(g_cover_badge_label, "MP3");
  lv_obj_set_style_text_color(g_cover_badge_label, lv_color_hex(0xFFFFFF), 0);
  lv_obj_align(g_cover_badge_label, LV_ALIGN_CENTER, 0, -10);

  g_cover_caption_label = lv_label_create(g_cover_card);
  lv_label_set_text(g_cover_caption_label, "Select a track");
  lv_obj_set_width(g_cover_caption_label, 90);
  lv_label_set_long_mode(g_cover_caption_label, LV_LABEL_LONG_WRAP);
  lv_obj_set_style_text_align(g_cover_caption_label, LV_TEXT_ALIGN_CENTER, 0);
  lv_obj_set_style_text_color(g_cover_caption_label, lv_color_hex(0xD1D5DB), 0);
  lv_obj_align(g_cover_caption_label, LV_ALIGN_CENTER, 0, 24);

  g_audio_title_label = lv_label_create(g_audio_page);
  lv_label_set_text(g_audio_title_label, "Audio idle");
  lv_obj_set_width(g_audio_title_label, 196);
  lv_obj_set_style_text_align(g_audio_title_label, LV_TEXT_ALIGN_CENTER, 0);
  lv_obj_set_style_text_color(g_audio_title_label, kTextColor, 0);
  lv_obj_set_pos(g_audio_title_label, 0, 118);

  g_audio_subtitle_label = lv_label_create(g_audio_page);
  lv_label_set_text(g_audio_subtitle_label, "Ready to play");
  lv_obj_set_width(g_audio_subtitle_label, 196);
  lv_obj_set_style_text_align(g_audio_subtitle_label, LV_TEXT_ALIGN_CENTER, 0);
  lv_obj_set_style_text_color(g_audio_subtitle_label, kMutedTextColor, 0);
  lv_obj_set_pos(g_audio_subtitle_label, 0, 136);

  g_audio_time_label = lv_label_create(g_audio_page);
  lv_label_set_text(g_audio_time_label, "00:00 / 00:00");
  lv_obj_set_width(g_audio_time_label, 196);
  lv_obj_set_style_text_align(g_audio_time_label, LV_TEXT_ALIGN_CENTER, 0);
  lv_obj_set_style_text_color(g_audio_time_label, kTextColor, 0);
  lv_obj_set_pos(g_audio_time_label, 0, 154);

  // Controls row
  lv_obj_t *controls = lv_obj_create(g_audio_page);
  lv_obj_remove_style_all(controls);
  lv_obj_set_size(controls, 196, 34);
  lv_obj_set_pos(controls, 0, 176);
  lv_obj_set_layout(controls, LV_LAYOUT_FLEX);
  lv_obj_set_flex_flow(controls, LV_FLEX_FLOW_ROW);
  lv_obj_set_flex_align(controls, LV_FLEX_ALIGN_SPACE_EVENLY, LV_FLEX_ALIGN_CENTER, LV_FLEX_ALIGN_CENTER);
  lv_obj_set_style_pad_all(controls, 0, 0);

  lv_obj_t *btn_prev = make_button(controls, 38, 32, LV_SYMBOL_PREV);
  lv_obj_add_event_cb(btn_prev, prev_track_event_cb, LV_EVENT_CLICKED, nullptr);

  lv_obj_t *btn_play = make_button(controls, 46, 34, LV_SYMBOL_PLAY);
  g_play_btn_label = lv_obj_get_child(btn_play, 0);
  lv_obj_set_style_bg_color(btn_play, kAccentColor, 0);
  lv_obj_add_event_cb(btn_play, play_pause_event_cb, LV_EVENT_CLICKED, nullptr);

  lv_obj_t *btn_next = make_button(controls, 38, 32, LV_SYMBOL_NEXT);
  lv_obj_add_event_cb(btn_next, next_track_event_cb, LV_EVENT_CLICKED, nullptr);

  lv_obj_t *btn_stop = make_button(controls, 38, 32, LV_SYMBOL_STOP);
  lv_obj_add_event_cb(btn_stop, stop_track_event_cb, LV_EVENT_CLICKED, nullptr);

  g_volume_label = lv_label_create(g_audio_page);
  lv_label_set_text(g_volume_label, "Vol: 12");
  lv_obj_set_width(g_volume_label, 196);
  lv_obj_set_style_text_align(g_volume_label, LV_TEXT_ALIGN_CENTER, 0);
  lv_obj_set_style_text_color(g_volume_label, kMutedTextColor, 0);
  lv_obj_set_pos(g_volume_label, 0, 214);

  lv_obj_t *volume_slider = lv_slider_create(g_audio_page);
  lv_obj_set_size(volume_slider, 150, 10);
  lv_slider_set_range(volume_slider, 0, 21);
  lv_slider_set_value(volume_slider, g_audio_volume, LV_ANIM_OFF);
  lv_obj_set_pos(volume_slider, 23, 234);
  lv_obj_add_event_cb(volume_slider, volume_slider_event_cb, LV_EVENT_VALUE_CHANGED, nullptr);

  lv_obj_add_flag(g_audio_page, LV_OBJ_FLAG_HIDDEN);
}

static void build_ui() {
  lv_obj_t *screen = lv_screen_active();

  lv_obj_set_style_bg_color(screen, kBgColor, 0);
  lv_obj_set_style_bg_opa(screen, LV_OPA_COVER, 0);

  lv_obj_t *panel = lv_obj_create(screen);
  lv_obj_remove_style_all(panel);
  lv_obj_set_size(panel, 216, 300);
  lv_obj_center(panel);
  lv_obj_set_style_bg_opa(panel, LV_OPA_TRANSP, 0);

  lv_obj_t *title = lv_label_create(panel);
  lv_label_set_text(title, "ESP32-S3 Assistant");
  lv_obj_set_width(title, 196);
  lv_obj_set_style_text_align(title, LV_TEXT_ALIGN_CENTER, 0);
  lv_obj_set_style_text_color(title, lv_color_hex(0xFFFFFF), 0);
  lv_obj_set_pos(title, 10, 0);

  lv_obj_t *subtitle = lv_label_create(panel);
  lv_label_set_text(subtitle, "Home + Music");
  lv_obj_set_width(subtitle, 196);
  lv_obj_set_style_text_align(subtitle, LV_TEXT_ALIGN_CENTER, 0);
  lv_obj_set_style_text_color(subtitle, kMutedTextColor, 0);
  lv_obj_set_pos(subtitle, 10, 20);

  g_home_tab_btn = make_button(panel, 84, 28, "Home");
  lv_obj_set_pos(g_home_tab_btn, 18, 46);
  lv_obj_add_event_cb(g_home_tab_btn, home_tab_event_cb, LV_EVENT_CLICKED, nullptr);
  lv_obj_set_style_bg_color(g_home_tab_btn, kActiveTabColor, 0);

  g_audio_tab_btn = make_button(panel, 84, 28, "Music");
  lv_obj_set_pos(g_audio_tab_btn, 114, 46);
  lv_obj_add_event_cb(g_audio_tab_btn, audio_tab_event_cb, LV_EVENT_CLICKED, nullptr);
  lv_obj_set_style_bg_color(g_audio_tab_btn, kIdleTabColor, 0);

  lv_obj_t *content = lv_obj_create(panel);
  lv_obj_remove_style_all(content);
  lv_obj_set_size(content, 196, 248);
  lv_obj_set_pos(content, 10, 86);
  lv_obj_set_style_bg_opa(content, LV_OPA_TRANSP, 0);

  build_home_page(content);
  build_audio_page(content);

  update_backlight_label(LCD_Backlight);
  update_volume_label();
  update_audio_ui();
  ui_refresh_timer_cb(nullptr);
  lv_timer_create(ui_refresh_timer_cb, 1000, nullptr);

  show_audio_page(false);
}

static void rtc_init_if_needed() {
  PCF85063_Init();

#if RTC_SET_TIME_ON_BOOT
  datetime_t now = {};
  now.year = 2026;
  now.month = 4;
  now.day = 7;
  now.dotw = 2;
  now.hour = 20;
  now.minute = 0;
  now.second = 0;
  PCF85063_Set_All(now);
#endif
}

static void sd_init_and_log() {
  SD_Init();

  uint8_t cardType = SD_MMC.cardType();
  if (cardType == CARD_NONE) {
    Serial.println("SD: no card detected after init");
    g_sd_ok = false;
    return;
  }

  g_sd_ok = true;

  Serial.printf("SD: mounted, size=%u MB\n", SDCard_Size);
  print_root_listing();
}

static void audio_init_v1() {
  audio.setPinout(I2S_BCLK, I2S_LRC, I2S_DOUT);
  audio.setVolume(g_audio_volume);
  Serial.println("Audio: V1 PCM5101 path initialized");
}

static void Driver_Init() {
  BAT_Init();
  I2C_Init();
  TCA9554PWR_Init(0x00);
  Backlight_Init();
  rtc_init_if_needed();
  sd_init_and_log();
  LCD_Init();
  Lvgl_Init();
  audio_init_v1();
}

void audio_info(const char *info) {
  Serial.print("audio_info: ");
  Serial.println(info);
}

void audio_showstreamtitle(const char *info) {
  Serial.print("audio_title: ");
  Serial.println(info);
}

void audio_eof_mp3(const char *info) {
  Serial.print("audio_eof_mp3: ");
  Serial.println(info);
  g_audio_paused = false;
  update_audio_ui();
}

void setup() {
  Serial.begin(115200);
  delay(500);

  unsigned long t0 = millis();
  while (!Serial && (millis() - t0) < 3000) {
    delay(10);
  }

  Serial.println();
  Serial.println("Booting ESP32-S3 Assistant...");

  Driver_Init();

  if (lvgl_port_lock(1000)) {
    build_ui();
    lvgl_port_unlock();
  }
}

void loop() {
  audio.loop();
  Lvgl_Loop();
}
