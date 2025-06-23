#!/bin/bash

if [[ -e "/tmp/tuun/tuun.lock" ]]; then
    echo "tuun is already running!" >&2
    exit 1
fi

cleanup() {
    rm -f "/tmp/tuun/quu.tpl"
    rm -f "/tmp/tuun/tuun.lock"
    pkill -x tuun
    pkill -x tuunfm
    [ -r "/tmp/tuun/tuun-mpv.pid" ] && kill "$(cat /tmp/tuun/tuun-mpv.pid)"
    tput cvvis
}

trap cleanup EXIT TERM

tput civis

if [[ -e /usr/libexec/tuun ]]; then
    TUUN_LOG_LEVEL="${TUUN_LOG_LEVEL:-debug}" /usr/libexec/tuun "$@"
else
    echo "Missing tuun at /usr/libexec/tuun" >&2
    echo "Did you run make install?" >&2
fi
