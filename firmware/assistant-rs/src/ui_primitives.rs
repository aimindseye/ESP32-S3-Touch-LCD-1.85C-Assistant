// RAW-V1-0-1-R12-UI-PRIMITIVES-MODULE
// Shared 360x360 RGB565 constants and low-level drawing primitives extracted from main.rs.

pub(crate) const W: usize = 360;
pub(crate) const H: usize = 360;
pub(crate) const PIXELS: usize = W * H;
pub(crate) const RGB565_ASSET_BYTES: usize = PIXELS * 2;
pub(crate) const UI_ASSET_COUNT: usize = 5;
pub(crate) const UI_ASSET_EMBEDDED_BYTES_REMOVED: usize = RGB565_ASSET_BYTES * UI_ASSET_COUNT;
pub(crate) const APP_BINARY_BEFORE_R7_BYTES: usize = 2_616_176;
pub(crate) const APP_PARTITION_BYTES: usize = 3_145_728;
pub(crate) const CX: i32 = 180;
pub(crate) const CY: i32 = 180;
pub(crate) const R_OUTER: i32 = 178;

pub(crate) const BLACK: u16 = 0x0000;
pub(crate) const BG: u16 = 0x0841;
pub(crate) const BG_DARK: u16 = 0x0008;
pub(crate) const BG_BLUE: u16 = 0x0296;
pub(crate) const RING: u16 = 0x18C3;
pub(crate) const RING_DIM: u16 = 0x1082;
pub(crate) const WHITE: u16 = 0xFFFF;
pub(crate) const MUTED: u16 = 0x9CF3;
pub(crate) const SOFT: u16 = 0x5AEB;

pub(crate) const ACCENT_HOME: u16 = 0x05DF;
pub(crate) const ACCENT_HOME_GREEN: u16 = 0x05E0;
pub(crate) const ACCENT_HOME_BLUE: u16 = 0x02DF;
pub(crate) const ACCENT_WEATHER: u16 = 0xFDE0;
pub(crate) const ACCENT_WEATHER_BLUE: u16 = 0x449F;
pub(crate) const ACCENT_MUSIC: u16 = 0x7818;
pub(crate) const ACCENT_MUSIC_BLUE: u16 = 0x22DF;
pub(crate) const ACCENT_ASSISTANT: u16 = 0x05FF;
pub(crate) const ACCENT_ASSISTANT_BLUE: u16 = 0x035F;
pub(crate) const ACCENT_SETTINGS: u16 = 0x44BF;
pub(crate) const STATUS_OK: u16 = 0x07E0;
pub(crate) const STATUS_BAD: u16 = 0xF800;

pub(crate) const TOP_STATUS_Y: i32 = 18;
pub(crate) const TITLE_Y: i32 = 70;
pub(crate) const SUBTITLE_Y: i32 = 112;
pub(crate) const FOOTER_DOTS_Y: i32 = 324;

pub(crate) fn inside_circle(x: i32, y: i32) -> bool {
    let dx = x - CX;
    let dy = y - CY;
    dx * dx + dy * dy <= R_OUTER * R_OUTER
}

pub(crate) fn set_pixel(frame: &mut [u16], x: i32, y: i32, color: u16) {
    if x >= 0 && y >= 0 && x < W as i32 && y < H as i32 && inside_circle(x, y) {
        frame[y as usize * W + x as usize] = color;
    }
}

pub(crate) fn fill_circle(frame: &mut [u16], cx: i32, cy: i32, r: i32, color: u16) {
    let rr = r * r;
    for y in (cy - r).max(0)..=(cy + r).min(H as i32 - 1) {
        for x in (cx - r).max(0)..=(cx + r).min(W as i32 - 1) {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= rr {
                set_pixel(frame, x, y, color);
            }
        }
    }
}

pub(crate) fn stroke_circle(frame: &mut [u16], cx: i32, cy: i32, r: i32, color: u16) {
    let outer = r * r;
    let inner = (r - 1) * (r - 1);
    for y in (cy - r).max(0)..=(cy + r).min(H as i32 - 1) {
        for x in (cx - r).max(0)..=(cx + r).min(W as i32 - 1) {
            let dx = x - cx;
            let dy = y - cy;
            let d = dx * dx + dy * dy;
            if d <= outer && d >= inner {
                set_pixel(frame, x, y, color);
            }
        }
    }
}

pub(crate) fn fill_rect(frame: &mut [u16], x: i32, y: i32, w: i32, h: i32, color: u16) {
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + w).min(W as i32);
    let y1 = (y + h).min(H as i32);

    for yy in y0..y1 {
        for xx in x0..x1 {
            set_pixel(frame, xx, yy, color);
        }
    }
}

pub(crate) fn stroke_rect(frame: &mut [u16], x: i32, y: i32, w: i32, h: i32, color: u16) {
    draw_line(frame, x, y, x + w - 1, y, color);
    draw_line(frame, x, y + h - 1, x + w - 1, y + h - 1, color);
    draw_line(frame, x, y, x, y + h - 1, color);
    draw_line(frame, x + w - 1, y, x + w - 1, y + h - 1, color);
}

pub(crate) fn stroke_rounded_rect(
    frame: &mut [u16],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    r: i32,
    color: u16,
) {
    // v0.1.13-r1 compile repair: Settings Option A used rounded-row outlines
    // but the raw renderer only had fill_rounded_rect. Keep this helper local and
    // primitive-based so no renderer/touch behavior changes are introduced.
    if w <= 0 || h <= 0 {
        return;
    }

    let radius = r.max(0).min(w / 2).min(h / 2);
    if radius == 0 {
        stroke_rect(frame, x, y, w, h, color);
        return;
    }

    draw_line(frame, x + radius, y, x + w - radius - 1, y, color);
    draw_line(
        frame,
        x + radius,
        y + h - 1,
        x + w - radius - 1,
        y + h - 1,
        color,
    );
    draw_line(frame, x, y + radius, x, y + h - radius - 1, color);
    draw_line(
        frame,
        x + w - 1,
        y + radius,
        x + w - 1,
        y + h - radius - 1,
        color,
    );

    draw_arc_segment(frame, x + radius, y + radius, radius, 1, 180, 270, color);
    draw_arc_segment(
        frame,
        x + w - radius - 1,
        y + radius,
        radius,
        1,
        270,
        360,
        color,
    );
    draw_arc_segment(
        frame,
        x + w - radius - 1,
        y + h - radius - 1,
        radius,
        1,
        0,
        90,
        color,
    );
    draw_arc_segment(
        frame,
        x + radius,
        y + h - radius - 1,
        radius,
        1,
        90,
        180,
        color,
    );
}

pub(crate) fn fill_rounded_rect(
    frame: &mut [u16],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    r: i32,
    color: u16,
) {
    fill_rect(frame, x + r, y, w - 2 * r, h, color);
    fill_rect(frame, x, y + r, w, h - 2 * r, color);
    fill_circle(frame, x + r, y + r, r, color);
    fill_circle(frame, x + w - r - 1, y + r, r, color);
    fill_circle(frame, x + r, y + h - r - 1, r, color);
    fill_circle(frame, x + w - r - 1, y + h - r - 1, r, color);
}

pub(crate) fn draw_chip(
    frame: &mut [u16],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    label: &str,
    accent: u16,
    selected: bool,
) {
    let bg = if selected { 0x2124 } else { 0x1082 };
    fill_rounded_rect(frame, x, y, w, h, h / 2, bg);
    stroke_round_chip(frame, x, y, w, h, h / 2, accent);
    draw_text_centered_at(frame, x + w / 2, y + h / 2 + 4, label, WHITE, 1);
}

pub(crate) fn stroke_round_chip(
    frame: &mut [u16],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    r: i32,
    color: u16,
) {
    draw_line(frame, x + r, y, x + w - r, y, color);
    draw_line(frame, x + r, y + h - 1, x + w - r, y + h - 1, color);
    draw_arc_segment(frame, x + r, y + r, r, 1, 180, 270, color);
    draw_arc_segment(frame, x + w - r - 1, y + r, r, 1, 270, 0, color);
    draw_arc_segment(frame, x + r, y + h - r - 1, r, 1, 90, 180, color);
    draw_arc_segment(frame, x + w - r - 1, y + h - r - 1, r, 1, 0, 90, color);
}

pub(crate) fn draw_ring_meter(
    frame: &mut [u16],
    cx: i32,
    cy: i32,
    r: i32,
    thickness: i32,
    progress: u8,
    base: u16,
    accent: u16,
) {
    draw_arc_segment(frame, cx, cy, r, thickness, 135, 45, base);
    let sweep = (progress.min(100) as i32 * 270) / 100;
    let end = (135 + sweep) % 360;
    draw_arc_segment(frame, cx, cy, r, thickness, 135, end, accent);
}

pub(crate) fn draw_arc_segment(
    frame: &mut [u16],
    cx: i32,
    cy: i32,
    r: i32,
    thickness: i32,
    start_deg: i32,
    end_deg: i32,
    color: u16,
) {
    let thickness = thickness.max(1);
    let half = thickness / 2;

    for offset in -half..=half {
        draw_arc_line(frame, cx, cy, r + offset, start_deg, end_deg, color);
    }
}

pub(crate) fn draw_arc_line(
    frame: &mut [u16],
    cx: i32,
    cy: i32,
    r: i32,
    start_deg: i32,
    end_deg: i32,
    color: u16,
) {
    let mut angle = normalize_deg(start_deg);
    let end = normalize_deg(end_deg);
    let mut guard = 0;
    let mut prev: Option<(i32, i32)> = None;

    loop {
        let rad = (angle as f32).to_radians();
        let point = (
            cx + (rad.cos() * r as f32).round() as i32,
            cy + (rad.sin() * r as f32).round() as i32,
        );

        if let Some((px, py)) = prev {
            draw_line(frame, px, py, point.0, point.1, color);
        } else {
            set_pixel(frame, point.0, point.1, color);
        }

        prev = Some(point);

        if angle == end || guard >= 360 {
            break;
        }

        angle = normalize_deg(angle + 1);
        guard += 1;
    }
}

pub(crate) fn normalize_deg(deg: i32) -> i32 {
    let mut value = deg % 360;
    if value < 0 {
        value += 360;
    }
    value
}

pub(crate) fn draw_line(frame: &mut [u16], mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: u16) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        set_pixel(frame, x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

pub(crate) fn fill_play_triangle(frame: &mut [u16], cx: i32, cy: i32, size: i32, color: u16) {
    let top = cy - size / 2;
    let bottom = cy + size / 2;
    let left = cx - size / 3;
    let right = cx + size / 2;

    for y in top..=bottom {
        let half = if y <= cy { y - top } else { bottom - y };
        let denom = (size / 2).max(1);
        let x_right = left + ((right - left) * half / denom);
        for x in left..=x_right {
            set_pixel(frame, x, y, color);
        }
    }
}

pub(crate) fn fill_left_triangle(frame: &mut [u16], cx: i32, cy: i32, size: i32, color: u16) {
    let top = cy - size / 2;
    let bottom = cy + size / 2;
    let right = cx + size / 3;
    let left = cx - size / 2;

    for y in top..=bottom {
        let half = if y <= cy { y - top } else { bottom - y };
        let denom = (size / 2).max(1);
        let x_left = right - ((right - left) * half / denom);
        for x in x_left..=right {
            set_pixel(frame, x, y, color);
        }
    }
}

pub(crate) fn draw_text_centered_at(
    frame: &mut [u16],
    cx: i32,
    y: i32,
    text: &str,
    color: u16,
    scale: i32,
) {
    let x = cx - text_width(text, scale) / 2;
    draw_text(frame, x, y, text, color, scale);
}

pub(crate) fn blit_rgb565_asset(frame: &mut [u16], asset: &[u8; RGB565_ASSET_BYTES]) {
    for (dst, px) in frame.iter_mut().zip(asset.chunks_exact(2)) {
        *dst = u16::from_le_bytes([px[0], px[1]]);
    }
}

pub(crate) fn draw_numeric_value_centered(
    frame: &mut [u16],
    y: i32,
    text: &str,
    digit_w: i32,
    stroke: i32,
    color: u16,
) {
    let width = numeric_value_width(text, digit_w, stroke);
    let mut x = CX - width / 2;

    for ch in text.chars() {
        let char_w = numeric_char_width(ch, digit_w, stroke);
        draw_numeric_char(frame, x, y, ch, digit_w, stroke, color);
        x += char_w + stroke;
    }
}

pub(crate) fn numeric_value_width(text: &str, digit_w: i32, stroke: i32) -> i32 {
    let mut width = 0;
    let mut first = true;

    for ch in text.chars() {
        if !first {
            width += stroke;
        }
        width += numeric_char_width(ch, digit_w, stroke);
        first = false;
    }

    width
}

pub(crate) fn numeric_char_width(ch: char, digit_w: i32, stroke: i32) -> i32 {
    match ch {
        ':' => stroke * 3,
        'F' | 'f' => digit_w - stroke * 2,
        '-' => digit_w / 2,
        _ => digit_w,
    }
}

pub(crate) fn draw_numeric_char(
    frame: &mut [u16],
    x: i32,
    y: i32,
    ch: char,
    digit_w: i32,
    stroke: i32,
    color: u16,
) {
    match ch {
        '0' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [true, true, true, true, true, true, false],
        ),
        '1' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [false, true, true, false, false, false, false],
        ),
        '2' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [true, true, false, true, true, false, true],
        ),
        '3' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [true, true, true, true, false, false, true],
        ),
        '4' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [false, true, true, false, false, true, true],
        ),
        '5' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [true, false, true, true, false, true, true],
        ),
        '6' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [true, false, true, true, true, true, true],
        ),
        '7' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [true, true, true, false, false, false, false],
        ),
        '8' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [true, true, true, true, true, true, true],
        ),
        '9' => draw_segment_mask(
            frame,
            x,
            y,
            digit_w,
            stroke,
            color,
            [true, true, true, true, false, true, true],
        ),
        ':' => {
            fill_circle(frame, x + stroke, y + digit_w / 2, stroke / 2 + 1, color);
            fill_circle(
                frame,
                x + stroke,
                y + digit_w + digit_w / 2,
                stroke / 2 + 1,
                color,
            );
        }
        'F' | 'f' => draw_letter_f(frame, x, y, digit_w, stroke, color),
        '-' => fill_rounded_rect(
            frame,
            x,
            y + digit_w,
            digit_w / 2,
            stroke,
            stroke / 2,
            color,
        ),
        _ => {}
    }
}

pub(crate) fn draw_segment_mask(
    frame: &mut [u16],
    x: i32,
    y: i32,
    w: i32,
    s: i32,
    color: u16,
    seg: [bool; 7],
) {
    let h = w * 2 - s;
    let mid = y + h / 2 - s / 2;
    let bottom = y + h - s;

    if seg[0] {
        fill_rounded_rect(frame, x + s, y, w - 2 * s, s, s / 2, color);
    }
    if seg[1] {
        fill_rounded_rect(frame, x + w - s, y + s, s, h / 2 - s, s / 2, color);
    }
    if seg[2] {
        fill_rounded_rect(frame, x + w - s, mid + s, s, h / 2 - s, s / 2, color);
    }
    if seg[3] {
        fill_rounded_rect(frame, x + s, bottom, w - 2 * s, s, s / 2, color);
    }
    if seg[4] {
        fill_rounded_rect(frame, x, mid + s, s, h / 2 - s, s / 2, color);
    }
    if seg[5] {
        fill_rounded_rect(frame, x, y + s, s, h / 2 - s, s / 2, color);
    }
    if seg[6] {
        fill_rounded_rect(frame, x + s, mid, w - 2 * s, s, s / 2, color);
    }
}

pub(crate) fn draw_letter_f(frame: &mut [u16], x: i32, y: i32, w: i32, s: i32, color: u16) {
    let h = w * 2 - s;
    fill_rounded_rect(frame, x, y, s, h, s / 2, color);
    fill_rounded_rect(frame, x, y, w - s, s, s / 2, color);
    fill_rounded_rect(frame, x, y + h / 2 - s / 2, w - s * 2, s, s / 2, color);
}

pub(crate) fn text_width(text: &str, scale: i32) -> i32 {
    text.chars().count() as i32 * 6 * scale
}

pub(crate) fn draw_text_centered(frame: &mut [u16], y: i32, text: &str, color: u16, scale: i32) {
    let x = CX - text_width(text, scale) / 2;
    draw_text(frame, x, y, text, color, scale);
}

pub(crate) fn draw_text(frame: &mut [u16], x: i32, y: i32, text: &str, color: u16, scale: i32) {
    let mut cursor_x = x;

    for ch in text.chars() {
        draw_char(frame, cursor_x, y, ch, color, scale);
        cursor_x += 6 * scale;
    }
}

pub(crate) fn draw_char(frame: &mut [u16], x: i32, y: i32, ch: char, color: u16, scale: i32) {
    let glyph = glyph_5x7(ch);

    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..5 {
            if ((bits >> (4 - col)) & 0x01) != 0 {
                fill_rect(
                    frame,
                    x + (col * scale),
                    y + (row as i32 * scale) - (7 * scale) + scale,
                    scale,
                    scale,
                    color,
                );
            }
        }
    }
}

pub(crate) fn glyph_5x7(ch: char) -> [u8; 7] {
    // RADIO_R34_LOWERCASE_ASCII_GLYPHS
    // The 5x7 table stores uppercase glyphs only. Convert ASCII
    // lowercase before matching so raw station labels such as
    // SanskarRadio and NonStopHindi do not render as S R / N S H.
    let ch = ch.to_ascii_uppercase();
    match ch {
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' => [0x1C, 0x12, 0x11, 0x11, 0x11, 0x12, 0x1C],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0E],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1F],
        'J' => [0x01, 0x01, 0x01, 0x01, 0x11, 0x11, 0x0E],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x1B, 0x11],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        ':' => [0x00, 0x04, 0x04, 0x00, 0x04, 0x04, 0x00],
        '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        '/' => [0x01, 0x01, 0x02, 0x04, 0x08, 0x10, 0x10],
        '%' => [0x18, 0x19, 0x02, 0x04, 0x08, 0x13, 0x03],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x04, 0x04, 0x08],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        '<' => [0x01, 0x02, 0x04, 0x08, 0x04, 0x02, 0x01],
        '>' => [0x10, 0x08, 0x04, 0x02, 0x04, 0x08, 0x10],
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        _ => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    }
}

// RAW-V1-0-1-R12-UI-PRIMITIVES-MODULE-END
