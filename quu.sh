#!/bin/bash

# [[ -e ~/.config/tuun/config.toml ]] || {
#     echo "Missing config at ~/.config/tuun/config.toml"
#     exit 1
# }

# TODO: Make this easily customizable from the config
SONG_DIR="$HOME/Music"
SCRIPTDIR="$(dirname "$(realpath "$0")")"

if [[ ! -d "$SONG_DIR" ]]; then
    echo "Music directory not found: $SONG_DIR"
    exit 1
fi

mapfile -t FULL_PATHS < <(find "$SONG_DIR" -type f \( -iname "*.mp3" -o -iname "*.opus" -o -iname "*.wav" -o -iname "*.m4a" \))
if [[ ${#FULL_PATHS[@]} -eq 0 ]]; then
    echo "No songs found in directory: $SONG_DIR"
    exit 1
fi

mapfile -t DISPLAY_NAMES < <(printf '%s\n' "${FULL_PATHS[@]}" | awk -F'/' '{print $NF}')
SHUFFLED_DISPLAY_NAMES=$(printf '%s\n' "${DISPLAY_NAMES[@]}" | shuf)

TMP_FILE=$(mktemp)
echo "$SHUFFLED_DISPLAY_NAMES" > "$TMP_FILE"

TMP_SCRIPT=$(mktemp)
chmod +x "$TMP_SCRIPT"

cat > "$TMP_SCRIPT" << 'EOF'
#!/bin/bash
cat "$1" | fzf --multi > "$2"
EOF

TMP_OUTPUT=$(mktemp)

alacritty --class "quu" -e "$TMP_SCRIPT" "$TMP_FILE" "$TMP_OUTPUT"
SELECTED_NAMES=$(cat "$TMP_OUTPUT" | sed "s@^@$SONG_DIR/@")
rm "$TMP_SCRIPT" "$TMP_OUTPUT"

if [[ -z "$SELECTED_NAMES" ]]; then
    echo "No songs selected"
    exit 0
fi

echo "$SELECTED_NAMES" > "/tmp/tuun/quu.tpl"

if ! pgrep -x 'tuun' > /dev/null 2>&1; then
    alacritty --class tuun --hold -e /usr/bin/tuun &
    exit 0
fi
