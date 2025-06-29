#!/bin/bash

# NOTE: This is just a reference implementation. Feel free to make it your own.

# Find the song directory
SONG_DIR=$(grep "music_dir = " ~/.config/tuun/config.toml | cut -d'"' -f2)
SONG_DIR=${SONG_DIR:-${XDG_MUSIC_DIR:-"$HOME/Music"}}

# And ensure it exists
if [[ ! -d "$SONG_DIR" ]]; then
    echo "Music directory not found: $SONG_DIR"
    exit 1
fi

# Gather selected songs
SEL="$(find "$SONG_DIR" -maxdepth 1 -mindepth 1 -type f \( -iname '*.mp3' -o -iname '*.opus' -o -iname '*.wav' -o -iname '*.m4a' -o -iname '*.ogg' -o -iname '*.flac' \) |
    sed 's,.*/,,'       | # strip full path
    shuf                | # shuffle
    fzm)"

[ -z "$SEL" ] && exit 0

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
