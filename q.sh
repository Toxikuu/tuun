#!/bin/bash
SONG_DIR="/home/t/Music"
MPV_SOCKET="/tmp/mpvsocket"

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

SELECTED_NAMES=$(printf '%s\n' "${SHUFFLED_DISPLAY_NAMES[@]}" | rofi -matching fuzzy -i -sort -theme-str 'window {width: 50%;}' -theme-str 'listview {columns: 1; lines: 15;}' -dmenu -multi-select -p "Queue songs:")
if [[ -z "$SELECTED_NAMES" ]]; then
    echo "No songs selected"
    exit 0
fi

SELECTED_NAMES=$(echo "$SELECTED_NAMES" | shuf)

while IFS= read -r SELECTED_NAME; do
    for i in "${!DISPLAY_NAMES[@]}"; do
        if [[ "${DISPLAY_NAMES[$i]}" == "$SELECTED_NAME" ]]; then
            ESCAPED_PATH=$(jq -R <<< "${FULL_PATHS[$i]}")
            echo "$ESCAPED_PATH"
            echo '{"command": ["loadfile", '"$ESCAPED_PATH"', "insert-next"]}' | socat - "$MPV_SOCKET"
            break
        fi
    done
done <<< "$SELECTED_NAMES"

echo "Selected songs have been inserted into the MPV queue"
