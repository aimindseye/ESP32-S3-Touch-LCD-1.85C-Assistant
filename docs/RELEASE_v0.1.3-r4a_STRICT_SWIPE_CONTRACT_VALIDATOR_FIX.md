# v0.1.3-r4a — Strict Swipe Contract Validator Fix

## Fix

v0.1.3-r4 had a packaging bug in `scripts/validate_rust_assistant_repo.ps1`: it still expected the old r3/r3a marker:

```text
SWIPE_THRESHOLD_PX: i16 = 90
```

The r4 strict contract intentionally requires:

```text
SWIPE_THRESHOLD_PX: i16 = 100
```

r4a updates the validator and adds a clearer guard for the r4 threshold.

## Preserved

- accepted v0.1.2 hardware baseline
- r3a UI polish
- r4 strict swipe-only page navigation
- center tap only for Select
- edge taps ignored
- top and bottom bands ignored
- POWER GPIO6 experimental logging
- BOOT reserved while USB monitor is attached
