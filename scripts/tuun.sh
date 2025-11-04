#!/usr/bin/env sh

# https://github.com/Toxikuu/tuun/issues/2
if [ -e "/tmp/tuun/tuun.lock" ]; then
    if pidof %LIBEXECDIR%/tuun; then
        echo "It looks like tuun is already running" >&2
        echo "If you're sure it's not, you can try removing /tmp/tuun/tuun.lock" >&2
        exit 1
    fi

    rm -vf /tmp/tuun/tuun.lock
fi

cleanup() {
    rm -f "/tmp/tuun/quu.tpl"
    rm -f "/tmp/tuun/tuun.lock"
    pkill -f %LIBEXECDIR%/tuun >/dev/null 2>&1
    [ -r "/tmp/tuun/tuun-mpv.pid" ] && kill "$(cat /tmp/tuun/tuun-mpv.pid)" >/dev/null 2>&1
    rm -f "/tmp/tuun/tuun-mpv.pid"
    tput cvvis
}

trap cleanup EXIT TERM

tput civis

if ! [ -e %LIBEXECDIR%/tuun ]; then
    echo "Missing tuun at %LIBEXECDIR%/tuun" >&2
    echo "Did you run make install?" >&2
    exit 1
fi

TUUN_LOG_LEVEL="${TUUN_LOG_LEVEL:-debug}" %LIBEXECDIR%/tuun "$@" &
wait $!
