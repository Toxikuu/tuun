#!/bin/bash

# NOTE: It is recommended you adjust this script, or make your own. It's
# tailored to how I want my song selection to work.

# NOTE: I use dmenu-flexipatch, and this script expects that. I've included the
# following patches:
#   - https://tools.suckless.org/dmenu/patches/fuzzyhighlight/
#   - https://tools.suckless.org/dmenu/patches/multi-selection/
#   - https://tools.suckless.org/dmenu/patches/numbers/
#   - https://tools.suckless.org/dmenu/patches/fuzzymatch/
#   - ctrl+v to paste
#   - https://tools.suckless.org/dmenu/patches/center/
#   - https://tools.suckless.org/dmenu/patches/colored-caret/
# Here's a link to a tarball of the patched sources:
# https://files.catbox.moe/w74y53.xz

set -x

# Find the song directory
SONG_DIR=$(grep "music_dir = " ~/.config/tuun/config.toml | cut -d'"' -f2)
SONG_DIR=${SONG_DIR:-"$HOME/Music"}

# And ensure it exists
if [[ ! -d "$SONG_DIR" ]]; then
    echo "Music directory not found: $SONG_DIR"
    exit 1
fi

# Gather selected songs
SEL="$(find "$SONG_DIR" -maxdepth 1 -mindepth 1 -type f \( -iname '*.mp3' -o -iname '*.opus' -o -iname '*.wav' -o -iname '*.m4a' -o -iname '*.ogg' -o -iname '*.flac' \) |
    sed 's,.*/,,'       | # strip full path
    shuf                | # shuffle
    dmenu -c -l 32 -i -fn "Iosevka Nerd Font-8"   ||
    exit 0)"

# Write them to the queue, and apply some fixes:
# 1. Prepend the song directory
# 2. Change \ to \\ to appease mpv
printf "%s\n" "${SEL[@]}"       |
    sed -e "s,^,$SONG_DIR/,"    \
        -e 's,\\,\\\\,g'        \
        > /tmp/tuun/quu.tpl

# Start tuun if it isn't running
if ! pgrep -x 'tuun' &> /dev/null; then
    alacritty --class tuun --hold -e /usr/bin/tuun &
fi
