# tuun

## Info
Tuun is my music player.
It is currently not intended to be compatible with systems that are not mine.
This is its fourth iteration, but the first iteration that's written in rust.
I accidentally deleted the previous three :((

It uses mpv under the hood, and the queue script uses rofi.

## Usage
Just run the binary, which will launch mpv, pointing at a (currently hardcoded) playlist.
I recommend adding custom keybinds in your window manager for the binary and the queue script.

## Dependencies
- mpv
- rofi (for q.sh)
- jq (needed for proper proper escaping in q.sh)
- tuunfm (optional, for tuunfm scrobbling)
