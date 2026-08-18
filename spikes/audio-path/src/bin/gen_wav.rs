use hound::{WavSpec, WavWriter, SampleFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let spec = WavSpec {
        channels: 2,
        sample_rate: 48000,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create("test_48k_stereo.wav", spec)?;

    let sample_rate = 48000.0_f32;
    let frequency = 340.0;
    let duration_secs = 3;
    let total_frames = sample_rate as usize * duration_secs;

    for n in 0..total_frames {
        let t = n as f32 / sample_rate;
        let value = (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.2;

        let sample_i16 = (value * i16::MAX as f32) as i16;

        writer.write_sample(sample_i16)?; // canal gauche
        writer.write_sample(sample_i16)?; // canal droit
    }

    // Indispensable pour observer une éventuelle erreur d'écriture du header final.
    writer.finalize()?;

    println!("Fichier écrit : test_48k_stereo.wav ({} s)", duration_secs);
    Ok(())
}
