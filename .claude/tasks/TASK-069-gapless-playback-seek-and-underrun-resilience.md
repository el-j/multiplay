---
id: TASK-069
title: Gapless Playback, Accurate Seeking & Underrun Resilience
planId: PLAN-022
status: pending
role: audio_engine
verificationCommand: cargo test -p amp-core-engine test_gapless
---

# TASK-069: Gapless Playback, Accurate Seeking & Underrun Resilience

## Description
Ensure audiophile playback stability across edge conditions:
1. Double-Buffering for Gapless Playback: Pre-open and pre-decode the next track's container header and initial audio frames 2 seconds before the current track reaches EOF, enabling sample-accurate, gapless track transitions.
2. Accurate Seeking: Implement precise seek positioning in Symphonia decoders by mapping target timestamps to packet stream presentation timestamps (PTS) and discarding pre-roll samples.
3. Underrun / Overrun Resilience: In case of slow network streams or system load, fill output buffers with soft-faded comfort silence rather than harsh digital clicks or pops.
4. Auto-Resume after Device Reconnect: Handle default audio output device switching (e.g. plugging/unplugging headphones or DACs) smoothly without application panic.

## Verification
```bash
cargo test -p amp-core-engine test_gapless
```

## Acceptance Criteria
- Gapless transition between consecutive tracks with zero latency gap or click.
- Accurate seeking to specified time offset.
- Clean silence insertion during buffer starving without clipping or panic.
