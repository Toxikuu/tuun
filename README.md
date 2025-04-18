# Tuun

## Info
Tuun is my music player. Compatibility isn't really a goal, but it should work
on most Linux distros with some finagling. This is its fifth iteration,
featuring async!

**Incomplete List of Dependencies**
- cargo -> Required. Used to compile Tuun.
- mpv -> Required. Used to play and control music and display album art.
- fzf -> Recommended. Necessary for queueing with quu.
- alacritty -> Recommended. You can use whatever terminal you like, but you'll
have to edit the shell scripts. Necessary for queueing with quu.

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
Run `tuun`.

***Pro tip:** You should make keybinds for tuun and quu in your window manager.*

## Uninstallation
If for whatever reason you're not convinced (and you still have the sources):
```bash
sudo make uninstall
```
