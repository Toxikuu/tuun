# Tuun

## Info
Tuun is my music player. Compatibility isn't really a goal, but it should work
on most Linux distros with some finagling. This is its fifth iteration,
featuring async!

**Incomplete List of Dependencies**
- rust -> Build
- mpv -> Runtime. Used to play and control music and display album art.
- dmenu (with patches) -> Optional. Used in `./quu.sh`.

## Features
Tuun currently supports at least the following fun and fancy features:
- Discord Rich Presence
- LastFM scrobbling
- Playlists
- Playlist generation
- Queues
- Configuration
- Global hotkeys

## Installation
I've decided to use Makefiles to simplify stuff. This should be all it takes:
```bash
make
sudo make install
```

## Usage
For basic usage, just run `tuun`. For more advanced usage, see below:

Thanks to mpv's socket, you can make hotkeys to control pretty much every aspect
of tuun.

Here's an example script of such a script. It may be easily combined with your
window manager to control tuun entirely with hotkeys.
```bash
#!/bin/bash

case "$1" in
    backward)
        echo '{ "command": ["seek", "-5", "relative", "exact"] }' | socat - /tmp/tuun/mpvsocket
        ;;
    forward)
        echo '{ "command": ["seek", "5", "relative", "exact"] }' | socat - /tmp/tuun/mpvsocket
        ;;
    previous)
        echo '{ "command": ["playlist-prev"] }' | socat - /tmp/tuun/mpvsocket
        ;;
    pause)
        echo '{ "command": ["cycle", "pause"] }' | socat - /tmp/tuun/mpvsocket
        ;;
    next)
        echo '{ "command": ["playlist-next"] }' | socat - /tmp/tuun/mpvsocket
        ;;
    loop)
        state=$(echo '{ "command": ["get_property", "loop-file"] }' | socat - /tmp/tuun/mpvsocket | jq -r '.data')

        if [ "$state" = "inf" ]; then
            echo '{ "command": ["set", "loop-file", "no"] }' | socat - /tmp/tuun/mpvsocket
        else
            echo '{ "command": ["set", "loop-file", "inf"] }' | socat - /tmp/tuun/mpvsocket
        fi
        ;;
    mute)
        echo '{ "command": ["cycle", "mute"] }' | socat - /tmp/tuun/mpvsocket
        ;;
    *)
        echo "Invalid option" >&2
        exit 1
        ;;
esac
```

You may also want to make keybinds and window class/title configurations for
`tuun` and `quu` with your window manager.

## Uninstallation
If for whatever reason you're not convinced (and you still have the sources):
```bash
sudo make uninstall
```
