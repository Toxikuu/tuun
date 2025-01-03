#!/bin/bash
SONG_DIR="$HOME/Music"
MPV_SOCKET="/tmp/mpvsocket"
SCRIPTDIR="$(dirname "$(realpath "$0")")"

if [[ ! -d "$SONG_DIR" ]]; then
    echo "Music directory not found: $SONG_DIR"
    exit 1
fi

# Collect available songs
mapfile -t FULL_PATHS < <(find "$SONG_DIR" -type f \( -iname "*.mp3" -o -iname "*.opus" -o -iname "*.wav" -o -iname "*.m4a" \))
if [[ ${#FULL_PATHS[@]} -eq 0 ]]; then
    echo "No songs found in directory: $SONG_DIR"
    exit 1
fi

# Start rofi
mapfile -t DISPLAY_NAMES < <(printf '%s\n' "${FULL_PATHS[@]}" | awk -F'/' '{print $NF}')
SHUFFLED_DISPLAY_NAMES=$(printf '%s\n' "${DISPLAY_NAMES[@]}" | shuf)
SELECTED_NAMES=$(printf '%s\n' "${SHUFFLED_DISPLAY_NAMES[@]}" | rofi -matching fuzzy -i -sort -theme-str 'window {width: 50%;}' -theme-str 'listview {columns: 1; lines: 15;}' -dmenu -multi-select -p "Queue songs:")

if [[ -z "$SELECTED_NAMES" ]]; then
    echo "No songs selected"
    exit 0
fi

# Shuffle the queue
SELECTED_NAMES=$(echo "$SELECTED_NAMES" | shuf)

# If q.sh is run before tuun is, play the selected song immediately on launch
if ! pgrep -x tuun; then
  alacritty --class tuun -e /code/tuun/run.sh &

  # Spam MPV's socket until it's ready
  # This is done to immediately play the selected song when launching from q.sh
  MAX_ATTEMPTS=500
  DELAY=0.005
  ATTEMPT=0

  while (( ATTEMPT < MAX_ATTEMPTS )); do
      if socat - "$MPV_SOCKET" <<< '{"command": ["get_property", "time-pos"]}' &> /dev/null; then
          break
      fi
      ((ATTEMPT++))
      sleep "$DELAY"
  done

  if (( ATTEMPT == MAX_ATTEMPTS )); then
      echo "Failed to connect to MPV socket after $MAX_ATTEMPTS attempts"
      exit 1
  fi

  FIRST_SELECTED=$(echo "$SELECTED_NAMES" | head -n 1)
  SELECTED_NAMES=$(echo "$SELECTED_NAMES" | tail -n +2)

  for i in "${!DISPLAY_NAMES[@]}"; do
      if [[ "${DISPLAY_NAMES[$i]}" == "$FIRST_SELECTED" ]]; then
          ESCAPED_PATH=$(jq -R <<< "${FULL_PATHS[$i]}")
          echo '{"command": ["loadfile", '"$ESCAPED_PATH"', "insert-next"]}' | socat - "$MPV_SOCKET"
          echo '{"command": ["playlist-next"]}' | socat - "$MPV_SOCKET"
          break
      fi
  done
fi

# Insert selected songs into the queue
while IFS= read -r SELECTED_NAME; do
    for i in "${!DISPLAY_NAMES[@]}"; do
        if [[ "${DISPLAY_NAMES[$i]}" == "$SELECTED_NAME" ]]; then
            ESCAPED_PATH=$(jq -R <<< "${FULL_PATHS[$i]}")
            echo '{"command": ["loadfile", '"$ESCAPED_PATH"', "insert-next"]}' | socat - "$MPV_SOCKET"
            break
        fi
    done
done <<< "$SELECTED_NAMES"

echo "Selected songs have been inserted into the MPV queue"
