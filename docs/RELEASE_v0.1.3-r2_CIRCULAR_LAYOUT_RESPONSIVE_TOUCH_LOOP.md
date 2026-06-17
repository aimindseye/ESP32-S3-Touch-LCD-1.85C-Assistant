# v0.1.3-r2 — Circular Layout Alignment and Responsive Touch Loop

## Scope
- Preserve accepted v0.1.2 hardware baseline
- Keep v0.1.3 circular UI direction
- Do not accept v0.1.3-r1 yet
- Align top status/header to fixed top band
- Align title under header consistently on all pages
- Align footer/page dots to fixed bottom band
- Rework Settings from left-heavy diagnostic block to centered watch tile/list
- Move dense diagnostics to secondary System Details view later
- Add non-blocking UI event loop
- Poll touch every 20–30 ms
- Implement coordinate-based touch down/move/up recognition
- Use swipe distance threshold instead of CST816 gesture dependency
- Keep left/center/right tap fallback
- Throttle RTC/log output so it does not affect responsiveness
- Keep POWER GPIO6 as experimental logging only
- Keep BOOT reserved while USB monitor is attached
- Keep build/flash helpers and validator

## Notes
- The top status band, title band, content area, and footer dots now use fixed vertical anchors.
- Touch interaction no longer depends on CST816 gesture bytes alone; release-time swipe/tap classification is added.
- Rendering is dirty-driven to improve responsiveness and reduce unnecessary panel redraws.
- RTC, battery, Wi-Fi, and SD refreshes are throttled on timers rather than tied to every UI pass.
