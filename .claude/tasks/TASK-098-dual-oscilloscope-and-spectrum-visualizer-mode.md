# TASK-098: Dual Oscilloscope & Spectrum Visualizer Mode

## Plan Reference
- Plan: [PLAN-030](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-030-native-file-dialogs-playlist-persistence-and-wsz-skins.md)
- Role: `dsp_engineering` & `frontend_gui`

## Objectives
1. Implement real-time phosphor green oscilloscope mode capturing raw PCM waveform in a lock-free circular buffer.
2. Allow clicking the visualizer window or main player display to toggle between 16-Band Spectrum Analyzer and Oscilloscope mode (classic Winamp behavior).
3. Implement authentic peak-fall and gravity decay physics for spectrum analyzer bars.
4. Add verification tests and ensure `make all` passes.

## Verification Command
`make all`

## Status
- **Status:** Pending
