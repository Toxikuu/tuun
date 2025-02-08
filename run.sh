#!/bin/bash

if grep -A1 tuunfm ~/.config/tuun/config.toml | grep -q true; then
  if ! pgrep -x tuunfm; then
    tuunfm &
  else
    echo "tuunfm is already running!" >&2
  fi
fi

if ! pgrep -x tuun; then
  tuun "$@"
else
  echo "tuun is already running!" >&2
fi
