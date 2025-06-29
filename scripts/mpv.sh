#!/usr/bin/env bash

# Script to control the mpv instance spawned by tuun from the command line

if ! command -v socat; then
    echo "Missing socat" >&2
    exit 1
fi

# Function to send a command to the mpv socket
send_command() {
    echo "{ \"command\": $1 }" | socat - /tmp/tuun/mpvsocket
}

case "$1" in
    backward)
        send_command '["seek", "-5", "relative", "exact"]'
        ;;
    forward)
        send_command '["seek", "5", "relative", "exact"]'
        ;;
    previous)
        send_command '["playlist-prev"]'
        ;;
    pause)
        send_command '["cycle", "pause"]'
        ;;
    next)
        send_command '["playlist-next"]'
        ;;
    loop)
        if send_command '["get_property", "loop-file"]' | grep inf; then
            send_command '["set", "loop-file", "no"]'
        else
            send_command '["set", "loop-file", "inf"]'
        fi
        ;;
    mute)
        send_command '["cycle", "mute"]'
        ;;
    *)
        echo "Invalid option" >&2
        exit 1
        ;;
esac
