## 5.2.1
- Make mpv socket poll timeout configurable, and change the default to 96 ms
- Update dependencies

## 5.2.0
- Command line arguments by @Stigstille
- Halved polling timeout for mpv's socket
- Marginally improve log output for mpv exits
- Formatting changes

## 5.1.0
- Improve discord ipc detection
- Improve latency surrounding discord rich presence
- Improve arturl detection for correctly-tagged mp3s
- Update dependencies
- Miscellaneous cleanup

## 5.0.0
- Partially revert to the old way of acquiring metadata
- Improve file format support
- Fix #2

## 4.2.0
- Improve handling of blocking scrobble tasks
- Update dependencies

## 4.1.3
- Escape double quotes in quu
- Update dependencies

## 4.1.2
- Attempt to fix a bug with tuun.sh
- Update dependencies

## 4.1.1
- Add release profile
- Address a cargo warning

## 4.1.0
- Drop unimplemented tuunfm support
- Rework scripts
- Use the permitit crate
- Misc fixes

## 4.0.0
- Hotkeys are no longer managed internally
- Updated dependencies
- Refactored
- Adapt to the metadata format used in [rip](https://git.gay/Tox/rip)
- This is not backwards compatible
    - Hotkeys are no longer supported
    - ID3 tag support only
