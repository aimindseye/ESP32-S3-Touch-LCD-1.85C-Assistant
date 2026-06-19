# v0.1.14-r2 Weather Guard Marker Repair

## Root cause

The v0.1.14 freeze guard had one stale Weather marker:

```text
draw_text_centered_at(frame, cx, 258, entry.temp, WHITE, 2)
```

The accepted Weather timeline code uses:

```text
draw_text_centered_at(frame, cx, 262, entry.temp, WHITE, 2)
```

## Scope

Validator-only repair:

```text
- no weather visual changes
- no RGB565 asset changes
- no touch changes
- preserve r1 stale-asset cleanup behavior
```
