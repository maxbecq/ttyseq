use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::RingBuffer;
use hound::WavReader;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- 1. Charger le fichier WAV en mémoire ---
    let mut reader = WavReader::open("test_48k_stereo.wav")?;
    let spec = reader.spec();
    println!("WAV : {} Hz, {} canaux, {} bits, {:?}",
        spec.sample_rate, spec.channels, spec.bits_per_sample, spec.sample_format);

    // On lit TOUS les échantillons dans un Vec<f32>.
    // samples::<f32>() renvoie un itérateur de Result<f32, _> ;
    // collect() rassemble tout, en propageant une éventuelle erreur de lecture.
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| {
            // chaque s est un Result<i16, _> ; on le déballe puis on convertit
            let sample_i16 = s.unwrap_or(0);
            sample_i16 as f32 / i16::MAX as f32 
        })
        .collect();
    println!("Chargé : {} échantillons", samples.len());

    // --- 2. Config audio ---
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("Aucun périphérique de sortie trouvé")?;
    let config = device.default_output_config()?;

    // Sécurité : on vérifie que le WAV correspond à la sortie (pas de resampling ici).
    if spec.sample_rate != config.sample_rate() {
        return Err(format!(
            "Sample rate WAV ({}) != sortie ({}). Resampling non géré dans ce spike.",
            spec.sample_rate, config.sample_rate()
        ).into());
    }

    // --- 3. Ring buffer ---
    let (mut producer, mut consumer) = RingBuffer::<f32>::new(8192);

    // --- 4. Thread audio : possède le consumer, ne fait que lire ---
    let stream_config = config.config();
    let out_channels = stream_config.channels as usize;   // 14 chez toi
    println!("StreamConfig : {:?}", stream_config);
    println!("Canaux de sortie : {}", out_channels);

    // Compteur d'underruns, partagé entre le thread audio (qui l'incrémente)
    // et le thread main (qui le lit). Arc = partage sûr entre threads.
    let underruns = Arc::new(AtomicUsize::new(0));
    let underruns_audio = Arc::clone(&underruns); // une "poignée" pour le thread audio

    let stream = device.build_output_stream(
        stream_config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(out_channels) {
                // pop() renvoie Result. Au lieu de unwrap_or muet,
                // on distingue le cas vide pour le COMPTER.
                let left = match consumer.pop() {
                    Ok(v) => v,
                    Err(_) => {
                        underruns_audio.fetch_add(1, Ordering::Relaxed);
                        0.0
                    }
                };
                let right = consumer.pop().unwrap_or(0.0);

                if out_channels >= 1 { frame[0] = left; }
                if out_channels >= 2 { frame[1] = right; }
                for extra in frame.iter_mut().skip(2) {
                    *extra = 0.0;
                }
            }
        },
        move |err| eprintln!("Erreur de flux : {}", err),
        None,
    )?;
    stream.play()?;    

    // --- 5. Thread main : pousse les échantillons du Vec dans le ring buffer ---
    println!("Lecture en boucle. Ctrl+C pour arrêter.");
    let mut index = 0;
    let start = std::time::Instant::now();
    let mut last_report = std::time::Instant::now();

    loop {
        if producer.push(samples[index]).is_ok() {
            index += 1;
            if index >= samples.len() {
                index = 0; // reboucle au début du fichier
            }
        } else {
            std::thread::sleep(std::time::Duration::from_micros(100));
        }

        // Toutes les 10 secondes, on affiche le compteur d'underruns.
        if last_report.elapsed() >= std::time::Duration::from_secs(10) {
            let count = underruns.load(Ordering::Relaxed);
            println!("[{:>4}s] underruns cumulés : {}",
                start.elapsed().as_secs(), count);
            last_report = std::time::Instant::now();
        }
    }   
}
