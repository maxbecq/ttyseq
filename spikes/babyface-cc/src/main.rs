// Babyface Pro (Class Compliant) x Raspberry Pi validation spike.
// Adapted from the frozen audio-path spike (spikes/audio-path/src/main.rs):
// same real-time path (cpal callback reading an rtrb ring buffer, stereo
// content on channels 1-2, remaining channels silenced), plus the knobs the
// protocol in doc/spikes/babyface-raspi.md needs: device selection, buffer
// size, channel count, sample rate and bounded run duration.
//
// Usage:
//   babyface-spike --list
//   babyface-spike [--device <name-substring>] [--buffer <frames>]
//                  [--rate <hz>] [--channels <n>] [--secs <n>] [--wav <path>]
//
// --secs 0 runs until Ctrl+C. Exits non-zero on any setup failure.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, StreamConfig};
use hound::WavReader;
use rtrb::RingBuffer;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct Opts {
    list: bool,
    device: String,
    buffer: u32,
    rate: u32,
    channels: u16,
    secs: u64,
    wav: String,
}

fn parse_opts() -> Result<Opts, String> {
    let mut opts = Opts {
        list: false,
        device: "Babyface".to_string(),
        buffer: 512,
        rate: 48_000,
        channels: 2,
        secs: 60,
        wav: "test_48k_stereo.wav".to_string(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut take = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--list" => opts.list = true,
            "--device" => opts.device = take("--device")?,
            "--buffer" => opts.buffer = take("--buffer")?.parse().map_err(|e| format!("--buffer: {e}"))?,
            "--rate" => opts.rate = take("--rate")?.parse().map_err(|e| format!("--rate: {e}"))?,
            "--channels" => opts.channels = take("--channels")?.parse().map_err(|e| format!("--channels: {e}"))?,
            "--secs" => opts.secs = take("--secs")?.parse().map_err(|e| format!("--secs: {e}"))?,
            "--wav" => opts.wav = take("--wav")?,
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(opts)
}

fn device_name(device: &cpal::Device) -> String {
    device
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| "<unknown>".to_string())
}

// ALSA PCM id (e.g. "plughw:CARD=Pro71993645,DEV=0"); distinguishes the many
// PCM aliases a single card exposes, which all share the same display name.
// The ALSA host puts it in the `driver` field of the description.
fn device_pcm_id(device: &cpal::Device) -> String {
    device
        .description()
        .ok()
        .and_then(|d| d.driver().map(str::to_string))
        .unwrap_or_default()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = parse_opts()?;
    let host = cpal::default_host();

    if opts.list {
        for device in host.output_devices()? {
            let default = device
                .default_output_config()
                .map(|c| format!("{} ch, {} Hz, {:?}", c.channels(), c.sample_rate(), c.sample_format()))
                .unwrap_or_else(|e| format!("no default config: {e}"));
            println!(
                "{:50} pcm={:45} [{}]",
                device_name(&device),
                device_pcm_id(&device),
                default
            );
        }
        return Ok(());
    }

    // --- 1. Load the WAV into memory (off the audio thread) ---
    let mut reader = WavReader::open(&opts.wav)?;
    let spec = reader.spec();
    println!(
        "WAV: {} Hz, {} channels, {} bits, {:?}",
        spec.sample_rate, spec.channels, spec.bits_per_sample, spec.sample_format
    );
    if spec.sample_rate != opts.rate {
        return Err(format!(
            "WAV sample rate ({}) != requested rate ({}). No resampling in this spike.",
            spec.sample_rate, opts.rate
        )
        .into());
    }
    if spec.channels != 2 {
        return Err("this spike expects a stereo WAV".into());
    }
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap_or(0) as f32 / i16::MAX as f32)
        .collect();
    println!("Loaded: {} samples", samples.len());

    // --- 2. Device selection by name substring ---
    let needle = opts.device.to_lowercase();
    let device = host
        .output_devices()?
        .find(|d| {
            device_name(d).to_lowercase().contains(&needle)
                || device_pcm_id(d).to_lowercase().contains(&needle)
        })
        .ok_or(format!("no output device matching \"{}\" (try --list)", opts.device))?;
    println!("Device: {} pcm={}", device_name(&device), device_pcm_id(&device));

    let stream_config = StreamConfig {
        channels: opts.channels,
        sample_rate: opts.rate,
        buffer_size: BufferSize::Fixed(opts.buffer),
    };
    println!("Requested config: {:?}", stream_config);
    let out_channels = stream_config.channels as usize;

    // --- 3. Ring buffer + underrun counter ---
    let (mut producer, mut consumer) = RingBuffer::<f32>::new(8192);
    let underruns = Arc::new(AtomicUsize::new(0));
    let underruns_audio = Arc::clone(&underruns);

    // --- 4. Audio thread: read-only, no alloc, no syscall, no panic ---
    let stream = device.build_output_stream(
        stream_config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(out_channels) {
                let left = match consumer.pop() {
                    Ok(v) => v,
                    Err(_) => {
                        underruns_audio.fetch_add(1, Ordering::Relaxed);
                        0.0
                    }
                };
                let right = consumer.pop().unwrap_or(0.0);
                if out_channels >= 1 {
                    frame[0] = left;
                }
                if out_channels >= 2 {
                    frame[1] = right;
                }
                for extra in frame.iter_mut().skip(2) {
                    *extra = 0.0;
                }
            }
        },
        move |err| eprintln!("stream error: {}", err),
        None,
    )?;
    stream.play()?;

    // --- 5. Main thread: feed the ring buffer, report every 10 s ---
    if opts.secs > 0 {
        println!("Playing for {} s...", opts.secs);
    } else {
        println!("Playing until Ctrl+C...");
    }
    let mut index = 0;
    let start = std::time::Instant::now();
    let mut last_report = std::time::Instant::now();

    loop {
        if producer.push(samples[index]).is_ok() {
            index += 1;
            if index >= samples.len() {
                index = 0;
            }
        } else {
            std::thread::sleep(std::time::Duration::from_micros(100));
        }

        if last_report.elapsed() >= std::time::Duration::from_secs(10) {
            println!(
                "[{:>5}s] cumulative underruns: {}",
                start.elapsed().as_secs(),
                underruns.load(Ordering::Relaxed)
            );
            last_report = std::time::Instant::now();
        }

        if opts.secs > 0 && start.elapsed().as_secs() >= opts.secs {
            break;
        }
    }

    drop(stream);
    println!(
        "RESULT buffer={} rate={} channels={} elapsed={}s underruns={}",
        opts.buffer,
        opts.rate,
        opts.channels,
        start.elapsed().as_secs(),
        underruns.load(Ordering::Relaxed)
    );
    Ok(())
}
