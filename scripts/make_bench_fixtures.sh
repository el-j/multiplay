#!/usr/bin/env bash
# Regenerates the 20-second files the resource-budget measurement plays (TASK-179). Music-like
# content (a chord plus quiet noise) so FLAC and MP3 decoding cost about what real albums cost;
# a pure sine would compress to almost nothing and flatter the numbers.
# ffmpeg is an offline tool for this script only; nothing in multiplay links it.
#
# Usage: ./scripts/make_bench_fixtures.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/crates/amp-adapter-gui/tests/fixtures/bench"
mkdir -p "$OUT"
command -v ffmpeg >/dev/null || { echo "missing ffmpeg" >&2; exit 1; }

SIGNAL="aevalsrc=0.2*sin(2*PI*220*t)+0.15*sin(2*PI*277.2*t)+0.12*sin(2*PI*329.6*t)|0.2*sin(2*PI*165*t)+0.15*sin(2*PI*247*t)+0.12*sin(2*PI*392*t):s=44100:d=20"
NOISE="anoisesrc=color=pink:amplitude=0.06:sample_rate=44100:duration=20:seed=7"
FF=(ffmpeg -hide_banner -loglevel error -y -f lavfi -i "$SIGNAL" -f lavfi -i "$NOISE"
    -filter_complex "[1]aformat=channel_layouts=stereo[n];[0][n]amix=inputs=2:normalize=0"
    -metadata title="Budget" -metadata artist="multiplay")

"${FF[@]}" -c:a flac -sample_fmt s16 "$OUT/bench_20s.flac"
"${FF[@]}" -c:a libmp3lame -b:a 256k "$OUT/bench_20s.mp3"
ls -la "$OUT"
