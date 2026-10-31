#!/usr/bin/env bash
# Regenerates the golden audio fixtures used by the decode matrix (TASK-226, PLAN-058).
#
# Every fixture carries the same known signal so tests can check the decoded samples:
#   left 440 Hz, right 660 Hz, amplitude 0.5 (5.1: 440/550/660/770/880/990 Hz).
# ffmpeg, flac, lame and python3 are offline tools for this script only; nothing in multiplay links them.
#
# Usage: ./scripts/make_audio_fixtures.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/crates/amp-adapter-decoder/tests/fixtures"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$OUT"

for tool in ffmpeg flac lame python3; do
    command -v "$tool" >/dev/null || { echo "missing $tool" >&2; exit 1; }
done

FF=(ffmpeg -hide_banner -loglevel error -y)
TAGS=(-metadata title="Fixture" -metadata artist="multiplay" -metadata album="Golden Fixtures")

# stereo <rate> <seconds> <output> [codec args...]
stereo() {
    local rate=$1 secs=$2 out=$3
    shift 3
    "${FF[@]}" -f lavfi -i "aevalsrc=0.5*sin(2*PI*440*t)|0.5*sin(2*PI*660*t):s=$rate:d=$secs" \
        "${TAGS[@]}" "$@" "$out"
}

# Lossless
stereo 44100 1 "$OUT/flac_16_44k.flac" -c:a flac -sample_fmt s16
stereo 96000 0.5 "$OUT/flac_24_96k.flac" -c:a flac -sample_fmt s32 -bits_per_raw_sample 24
stereo 192000 0.25 "$OUT/flac_24_192k.flac" -c:a flac -sample_fmt s32 -bits_per_raw_sample 24
stereo 44100 1 "$OUT/wav_16.wav" -c:a pcm_s16le
stereo 48000 0.5 "$OUT/wav_24.wav" -c:a pcm_s24le
stereo 48000 0.5 "$OUT/wav_f32.wav" -c:a pcm_f32le

stereo 44100 1 "$TMP/plain.wav" -c:a pcm_s16le
flac --silent --force --no-seektable -T TITLE=Fixture -T ARTIST=multiplay \
    -o "$OUT/flac_no_seektable.flac" "$TMP/plain.wav"

"${FF[@]}" -f lavfi -i "aevalsrc=0.5*sin(2*PI*440*t)|0.5*sin(2*PI*550*t)|0.5*sin(2*PI*660*t)|0.5*sin(2*PI*770*t)|0.5*sin(2*PI*880*t)|0.5*sin(2*PI*990*t):s=48000:d=0.5:c=5.1" \
    "${TAGS[@]}" -c:a flac -sample_fmt s16 "$OUT/flac_5_1_48k.flac"

# Lossy
lame --silent -b 128 --tt Fixture --ta multiplay "$TMP/plain.wav" "$OUT/mp3_cbr_128.mp3"
lame --silent -V 2 --tt Fixture --ta multiplay "$TMP/plain.wav" "$OUT/mp3_vbr.mp3"
stereo 44100 1 "$OUT/aac.m4a" -c:a aac -b:a 128k
stereo 44100 1 "$OUT/vorbis.ogg" -c:a vorbis -strict -2 -ac 2

# Formats enabled by TASK-227
stereo 44100 1 "$OUT/alac.m4a" -c:a alac -sample_fmt s16p
stereo 44100 1 "$OUT/aiff_16.aiff" -c:a pcm_s16be -write_id3v2 1
stereo 44100 1 "$OUT/caf_16.caf" -c:a pcm_s16le -f caf
stereo 44100 1 "$OUT/flac.mka" -c:a flac -sample_fmt s16 -f matroska
stereo 44100 1 "$OUT/vorbis.webm" -c:a vorbis -strict -2 -ac 2 -f webm
stereo 44100 1 "$TMP/raw.mp2" -c:a mp2 -b:a 192k
# ffmpeg's mp2 muxer writes no tags: prepend an ID3v2.3 tag, as tagged .mp2 files have.
python3 - "$TMP/raw.mp2" "$OUT/mp2_192.mp2" <<'PY'
import sys
def frame(fid, text):
    data = b"\x03" + text.encode() + b"\x00"
    return fid.encode() + len(data).to_bytes(4, "big") + b"\x00\x00" + data
body = frame("TIT2", "Fixture") + frame("TPE1", "multiplay") + frame("TALB", "Golden Fixtures")
size = len(body)
syncsafe = bytes([(size >> 21) & 0x7F, (size >> 14) & 0x7F, (size >> 7) & 0x7F, size & 0x7F])
audio = open(sys.argv[1], "rb").read()
open(sys.argv[2], "wb").write(b"ID3\x03\x00\x00" + syncsafe + body + audio)
PY
stereo 44100 1 "$OUT/adpcm_ms.wav" -c:a adpcm_ms

# Opus (TASK-229): always decoded at 48 kHz
stereo 48000 1 "$OUT/opus_stereo.opus" -c:a libopus -b:a 128k
"${FF[@]}" -f lavfi -i "aevalsrc=0.5*sin(2*PI*440*t):s=48000:d=1" "${TAGS[@]}" \
    -c:a libopus -b:a 64k "$OUT/opus_mono.opus"
"${FF[@]}" -f lavfi -i "aevalsrc=0.5*sin(2*PI*440*t)|0.5*sin(2*PI*550*t)|0.5*sin(2*PI*660*t)|0.5*sin(2*PI*770*t)|0.5*sin(2*PI*880*t)|0.5*sin(2*PI*990*t):s=48000:d=0.5:c=5.1" \
    "${TAGS[@]}" -c:a libopus -mapping_family 1 -b:a 256k "$OUT/opus_5_1.opus"
stereo 48000 1 "$OUT/opus.webm" -c:a libopus -b:a 128k -f webm

# WavPack (TASK-231): lossless; ffmpeg appends an APEv2 tag like most real .wv files have.
stereo 44100 1 "$OUT/wavpack_16.wv" -c:a wavpack -sample_fmt s16p
stereo 48000 0.5 "$OUT/wavpack_24.wv" -c:a wavpack -sample_fmt s32p -bits_per_raw_sample 24

# Rich tags and embedded cover art (TASK-230); the cover is the 64 px app icon.
COVER="$ROOT/assets/icon/multiplay-64.png"
RICH=(-metadata album_artist="Various Fixtures" -metadata genre="Test Tones" -metadata date=2024
    -metadata disc=2 -metadata track=3 -metadata REPLAYGAIN_TRACK_GAIN="-6.50 dB")
for ext in flac mp3 m4a; do
    "${FF[@]}" -f lavfi -i "aevalsrc=0.5*sin(2*PI*440*t)|0.5*sin(2*PI*660*t):s=44100:d=0.5" -i "$COVER" \
        -map 0 -map 1 -c:v copy -disposition:v attached_pic "${TAGS[@]}" "${RICH[@]}" "$OUT/cover.$ext"
done

python3 "$ROOT/scripts/make_dsd_fixtures.py" "$OUT"

ls -l "$OUT"
