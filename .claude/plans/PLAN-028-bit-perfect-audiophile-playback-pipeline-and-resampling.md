# PLAN-028: Bit-Perfect Audiophile Playback Pipeline & Hardware Sinc Resampling

## Overview & Objective
Transform the playback audio pipeline in `amp-adapter-gui`, `amp-app`, and `amp-adapter-decoder` into a bit-perfect, clip-free, sample-rate independent audiophile engine.
Currently, audio files are decoded at their source sample rate without dynamic resampling to the hardware output rate, causing pitch/speed distortion (+1.088x on 44.1kHz -> 48kHz). Boosted EQ curves lack peak limiting, risking digital clipping. Multi-channel audio drops center and surround channels, and ring buffer saturation causes CPU spinning.

## Technical Architecture & Design
1. **Dynamic Sinc Resampling Engine:**
   - In `AudioPlaybackDriver`, query `dec.sample_rate()` upon track open.
   - If `source_sample_rate != sink_sample_rate`, instantiate `PureResampler` (`rubato` polyphase sinc).
   - Dynamically resample decoded chunks to `sink_sample_rate` before feeding EQ and ring buffer.
   - If `source_sample_rate == sink_sample_rate`, bypass resampler for bit-perfect passthrough.
2. **Soft-Knee Peak Limiter & Headroom Safeguard:**
   - Integrate `PureVolumeNormalizer` with ITU-R BS.1770 / EBU R128 soft-knee compression in the audio worker loop.
   - Clamp maximum true-peak output to -0.5 dBFS, guaranteeing zero inter-sample distortion regardless of EQ/preamp boosts.
3. **ITU-R BS.775 Channel Downmixer Matrix:**
   - In `AudioStreamDecoder`, implement compliant downmixing:
     - Mono: $L = M, R = M$
     - Stereo: Passthrough
     - 5.1 / 7.1 Surround: $L = L + 0.7071 \cdot C + 0.7071 \cdot SL$; $R = R + 0.7071 \cdot C + 0.7071 \cdot SR$
4. **Adaptive Lock-Free Buffer Backpressure:**
   - Replace CPU-burning `std::thread::yield_now()` spinning on full ring buffers with smart micro-sleeps based on remaining buffer capacity, lowering audio worker CPU usage to < 0.5%.

## Tasks Breakdown
- **TASK-087**: Live Polyphase Sinc Resampling Engine (`rubato` in audio drivers)
- **TASK-088**: Soft-Knee Limiter & Headroom Safeguard (zero digital clipping on EQ boost)
- **TASK-089**: ITU-R BS.775 Channel Matrix & Downmixer in Symphonia decoder
- **TASK-090**: Adaptive Lock-Free Buffer Backpressure & Audio Thread Hardening
