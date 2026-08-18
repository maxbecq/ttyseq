# babyface-cc-spike

Hardware validation spike for the RME Babyface Pro in Class Compliant mode on
Raspberry Pi, driving the protocol in `doc/spikes/babyface-raspi.md` (steps
6-8). Adapted from the frozen `spikes/audio-path/` reference: same real-time
path (cpal output callback reading an `rtrb` ring buffer, stereo WAV content on
channels 1-2, remaining channels silenced), plus the knobs the protocol needs.

Like `audio-path`, this is throwaway validation code: excluded from the future
Cargo workspace, built and run from this directory.

## Run

```sh
cargo run --bin babyface-spike -- --list
cargo run --bin babyface-spike -- --device "plughw:CARD=Pro71993645" \
    --buffer 512 --rate 48000 --channels 12 --secs 60 --wav test_48k_stereo.wav
```

`--device` matches a substring of the device name or of the ALSA PCM id shown
by `--list`. `--secs 0` runs until Ctrl+C. The test WAV is a git-ignored
artifact: generate it with `cargo run --bin gen_wav` in `spikes/audio-path/`
(or copy it from there).

## Notes (CC mode, from the spike session)

- The raw `hw:` PCM only accepts S24_3LE; cpal streams f32, so the spike goes
  through `plughw:` and lets ALSA convert.
- A single ALSA card shows up as many cpal devices (one per PCM alias:
  `plughw`, `default`, `sysdefault`, `front`, `surround*`, ...) that share the
  same display name; the PCM id lives in `DeviceDescription::driver()`.
