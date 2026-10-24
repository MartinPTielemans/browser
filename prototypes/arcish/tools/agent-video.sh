#!/bin/sh
# Turns a recording from tools/agent-demo.js into an mp4: tools/agent-video.sh <out-dir> [speed]
set -e
dir="$1"
speed="${2:-1}"
ffmpeg -y -loglevel error -f concat -safe 0 -i "$dir/frames.txt" \
  -vf "setpts=PTS/$speed,scale=1360:-2,fps=30,format=yuv420p" -c:v libx264 -crf 24 -movflags +faststart "$dir/demo.mp4"
echo "$dir/demo.mp4"
