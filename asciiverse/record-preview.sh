#!/bin/sh
# Records the catalog preview, asciiverse/aquest.cast, to hub/RECORDING.md:
# 100x30, the built-in demo (xquest-sdl's reference recording) in Mixed,
# switching to Blocks about a third of the way in and back to Mixed (V
# twice, through Braille) at two thirds.
# Runs asciinema in a detached tmux pane, so nothing from this terminal
# gets in; a throwaway HOME and data folder keep the settings untouched.
#
#   cargo build --release && asciiverse/record-preview.sh
set -eu
cd "$(dirname "$0")/.."
out=asciiverse/aquest.cast
tmp=$(mktemp -d)
trap 'tmux kill-session -t aquest-rec 2>/dev/null || true; rm -rf "$tmp"' EXIT
rm -f "$out"
tmux new-session -d -s aquest-rec -x 100 -y 30 \
  "env -i PATH=$PWD/target/release:/usr/bin:/bin HOME=$tmp USER=player LOGNAME=player \
     AQUEST_DATA_DIR=$tmp TERM=xterm-256color COLORTERM=truecolor LANG=C.UTF-8 \
   asciinema rec --window-size 100x30 --idle-time-limit 2 \
     --capture-env TERM,COLORTERM,LANG --title AQuest \
     --command 'aquest --demo --mute --mixed' $out"
sleep 18
tmux send-keys -t aquest-rec v
sleep 16
tmux send-keys -t aquest-rec v v
sleep 16
tmux send-keys -t aquest-rec Escape
sleep 2
ls -la "$out"
