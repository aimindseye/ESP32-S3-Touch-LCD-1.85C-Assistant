# v0.1.3-r3a — Touch Classifier and Watch UI Polish Build Fix

## Fix

v0.1.3-r3 used `last_render` before declaring it during the initial render. r3a moves:

```rust
let mut last_render = Instant::now() - Duration::from_millis(RENDER_MIN_INTERVAL_MS);
```

above the first `render_if_dirty(...)` call.

## Preserved

- accepted v0.1.2 hardware baseline
- v0.1.3-r3 watch UI polish
- removed divider lines
- dim/thin outer ring
- centered Settings watch tile
- touch classifier rules
- 300ms navigation cooldown
- coalesced dirty rendering
- POWER GPIO6 experimental logging
- BOOT reserved while USB monitor is attached
