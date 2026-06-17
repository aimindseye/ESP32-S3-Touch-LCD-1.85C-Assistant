# v0.1.3-r3 — Touch Classifier and Watch UI Polish

## Status

v0.1.3-r2a is not accepted. This r3 patch targets the two remaining issues from hardware testing: display polish and inconsistent/delayed touch navigation.

## Scope

- Preserve accepted v0.1.2 hardware baseline
- Do not accept v0.1.3-r2a yet
- Keep circular watch-style UI direction
- Remove top and bottom divider lines
- Dim/thin outer ring
- Move status dots slightly inward and upward-safe
- Rework Settings into centered list-style watch tile
- Move dense diagnostics to future System Details page
- Stop repainting every RTC tick on non-Home pages
- Coalesce dirty renders and avoid rendering during active touch
- Add touch navigation cooldown
- Classify tap only when movement <= 25px
- Classify swipe only when abs(dx) >= 90px and abs(dx) > 2 * abs(dy)
- Use tap zones only for true taps, not moved touches
- Keep center tap as Select
- Keep POWER GPIO6 experimental logging only
- Keep BOOT reserved while USB monitor is attached
- Keep build/flash helpers and validator

## Runtime expectations

- Small accidental movements should not navigate.
- Center taps should not become NextPage because release ends past x=240.
- Page changes require either a true left/right tap or deliberate horizontal swipe.
- Repaint should not occur while touch is active.
- RTC updates should only repaint Home, reducing diagnostic-style redraw noise on Weather/Music/Settings.
