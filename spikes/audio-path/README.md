# Audio path spike — frozen reference

Imported from [`maxbecq/ttyseq-spike`](https://github.com/maxbecq/ttyseq-spike)
at commit `7cb1390` (2026). The original repository remains the archive of record
for the full history.

## Status

**Frozen reference code — not production code.** This spike exists to de-risk
the real-time audio path and the Norns Shield screen before committing to
abstractions in ttySeq proper. Validated patterns graduate into the real
codebase by being rewritten there; this code is not maintained or evolved.
It is kept in-repo because it doubles as a hardware smoke-test tool
(see `doc/spikes/babyface-raspi.md`).

It is intentionally excluded from the future Cargo workspace: build and run it
from this directory.

## What it validated

- Real-time audio pipeline: cpal (0.18) output stream + `rtrb` lock-free SPSC
  ring buffer, audio thread strictly read-only (no alloc, no syscall, no panic).
- WAV playback via `hound` (f32 and i16 sources), underrun counting via atomics.
- Multichannel routing: stereo content on channels 1-2, remaining channels
  silenced — validated on a 14-channel RME Babyface Pro and on the Pi's
  2-channel output.
- **RT stability: 60 min / 0 underruns on Raspberry Pi 4 (debug build).**
- SSD1322 OLED (Norns Shield) over SPI: raw driver + `embedded-graphics`
  DrawTarget. Buttons/encoders not yet tested.

Details, RT rules and environment gotchas (e.g. `default_output_config()`
channel-count mismatch): see [`CLAUDE.md`](CLAUDE.md) in this directory.

## Contents

| Path | Purpose |
|---|---|
| `src/main.rs` | Main playback binary: WAV → ring buffer → cpal callback, underrun counter |
| `src/bin/gen_wav.rs` | Generates the local test WAV files (`*.wav` are git-ignored artifacts) |
| `src/bin/eg_probe.rs` | SSD1322 screen via `embedded-graphics` (Shield/Fates, Linux only) |
| `src/bin/fates_probe.rs` | SSD1322 screen probe (Shield/Fates, Linux only) |
| `src/bin/zjy_probe.rs` | SSD1322 screen probe, ZJY module variant (Linux only) |

## Run

```sh
cd spikes/audio-path
cargo run --bin gen_wav                   # recreate test WAV(s) first
cargo run --bin ttyseq-spike              # play through the ring buffer
cargo run --release --bin ttyseq-spike    # for RT / stability testing
```

The screen probes use Linux-only crates (`spidev`, `gpiod`) and are gated
behind the `shield` cargo feature — same compile-time model as the future
`norns-shield` feature (spec §3.3.7). On the Pi:

```sh
cargo run --bin fates_probe --features shield
```

Without the feature the package builds on macOS (audio binaries only).

The playback binary refuses a WAV whose sample rate differs from the output
device (no resampling in the spike — consistent with the ttySeq MVP policy,
cf. `doc/spec/data-model.md §5`).
