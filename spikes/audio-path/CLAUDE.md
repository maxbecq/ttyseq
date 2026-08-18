# CLAUDE.md — ttyseq-spike

## Purpose

Throwaway spike to validate the real-time audio pipeline for **ttySeq** (a
terminal sequencer for live audio/MIDI/CV performance). This repo is **not**
production code and is **not** the future crate structure. Its only job is to
de-risk the audio path before committing to abstractions.

Guiding principle: **spike before abstracting**. Do not introduce library
crates, traits, or premature abstractions here. When a pattern is validated,
it graduates to the real ttySeq workspace — it does not get "cleaned up" into
architecture inside this repo.

## What has been validated

- Device enumeration and default output config (cpal 0.18.1).
- Sine generation inside a real-time callback.
- Lock-free transport between threads via an `rtrb` ring buffer.
- WAV playback via `hound` 3.5.1, both f32 and 16-bit int (with i16 -> f32
  conversion at load time).
- Multichannel output routing: stereo content mapped to the first two output
  channels, remaining channels silenced (validated on a 14-channel RME
  Babyface and on the Pi's 2-channel output).
- **RT stability: 60 min / 0 underruns on Raspberry Pi 4, debug build.**

## Real-time rules (non-negotiable in the audio callback)

- No allocation, no syscalls, no blocking, no locks.
- No `.unwrap()` / `.expect()` (they panic). Use deterministic fallbacks
  instead: `.unwrap_or(0.0)` on an empty ring buffer yields silence.
- Underruns are counted (atomic), never fatal.
- All faillible/heavy work (file loading, decoding) happens off the audio
  thread. The audio thread only reads the ring buffer.

## Build & run

Native compilation on each target (macOS for dev, Pi 4 for RT validation).

```
cargo run --bin gen_wav          # generate the test WAV(s)
cargo run --bin ttyseq-spike     # play through the ring buffer
cargo run --release --bin ttyseq-spike   # for serious RT / stability testing
```

Note: test `.wav` files are generated artifacts, not versioned. Run `gen_wav`
after cloning/pulling to recreate them locally.

## Known environment constraints (surfaced by the spike)

- `default_output_config()` may report a different channel count than the
  actual stream `StreamConfig` (Babyface reported 2 by default but the stream
  opened 14). Always read channel count from the stream config, not the
  default config.
- Output device selection and per-channel routing are real concerns for
  ttySeq (Pi outputs vs Fates WM8731 vs USB interfaces). Out of scope for the
  spike — noted for the real project.

## Dependencies

- `cpal` 0.18.1 — audio I/O. Note: `Device::name()` was removed from
  `DeviceTrait`; use `device.description()?.name()`.
- `rtrb` — lock-free SPSC ring buffer.
- `hound` 3.5.1 — WAV read/write.

## Conventions

- Conventional Commits.
- Code, comments, commits in English.
