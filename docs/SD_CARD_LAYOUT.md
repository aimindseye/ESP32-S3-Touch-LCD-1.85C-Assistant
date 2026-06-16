# SD Card Layout

Recommended SD card layout for the Assistant firmware.

## Phase 0

The reference player currently supports MP3 playback from SD. For Assistant, place tracks under `/MUSIC`.

```text
/MUSIC/
  TRACK001.MP3
  TRACK002.MP3
  TRACK003.MP3
```

During the first import, the original hardcoded root-level tracks may still be present. The first Music refactor should scan `/MUSIC` dynamically.

## Future Layout

```text
/ASSIST/
  SETTINGS.JSN
  WEATHER.JSN
  STATE.JSN

/MUSIC/
  TRACK001.MP3
  TRACK002.MP3
  ALBUM1/
    TRACK001.MP3
    TRACK002.MP3

/PLAYLIST/
  FAVORITE.M3U
  MORNING.M3U
```

Use short, simple filenames at first to avoid UI wrapping and path handling issues.
