#!/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cleanup() {
    rm -f "$HOME/Music/Playlists/queue.tpl"
    rm -f "/tmp/tuun.lock"
    pkill -x tuun
    pkill -x tuunfm
    tput cvvis
}

trap cleanup EXIT

[[ -e "/tmp/tuun.lock" ]] && {
    echo "tuun is already running!" >&2
    exit 1
}

tput civis
"$SCRIPT_DIR"/target/release/tuun "$@"
