use flacenc::component::BitRepr;
use flacenc::error::Verify;
use loom_core::{decode_track, decode_track_partial, encode_session, encode_track_with_config, EncoderConfig};
use std::time::Instant;

fn generate_sine_sweep(length: usize, sample_rate: u32) -> Vec<i64> {
    let mut data = Vec::with_capacity(length);
    for i in 0..length {
        let t = i as f64 / sample_rate as f64;
        let freq = 100.0 + (2000.0 - 100.0) * (i as f64 / length as f64);
        let val = (16383.0 * (2.0 * std::f64::consts::PI * freq * t).sin()).round() as i64;
        data.push(val);
    }
    data
}

fn generate_drum_transient(length: usize, sample_rate: u32) -> Vec<i64> {
    let mut data = Vec::with_capacity(length);
    for i in 0..length {
        let env = (-5.0 * (i as f64 / (sample_rate as f64 * 0.1))).exp();
        let noise = (fastrand::i16(..) as f64) / 32768.0;
        let val = (noise * env * 24000.0).round() as i64;
        data.push(val);
    }
    data
}

fn generate_white_noise(length: usize) -> Vec<i64> {
    (0..length).map(|_| fastrand::i16(..) as i64).collect()
}

fn generate_silence(length: usize) -> Vec<i64> {
    vec![0i64; length]
}

fn encode_flac(samples: &[i32], sample_rate: u32, block_size: usize) -> Vec<u8> {
    let config = flacenc::config::Encoder::default().into_verified().unwrap();
    let source = flacenc::source::MemSource::from_samples(samples, 1, 16, sample_rate as usize);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, block_size).unwrap();
    let mut sink = flacenc::bitsink::MemSink::default();
    stream.write(&mut sink).unwrap();
    sink.as_slice().to_vec()
}

fn main() {
    let sample_rate = 44100;
    let seconds = 5;
    let length = (sample_rate * seconds) as usize;
    let raw_bytes = length * 2;

    println!("================================================================================");
    println!("             LOOM CODEC & SESSION CONTAINER COMPREHENSIVE BENCHMARK              ");
    println!("================================================================================");
    println!("Test Signal Duration : {} seconds ({} samples, {} bytes raw PCM per track)", seconds, length, raw_bytes);
    println!("Sample Format        : 16-bit PCM Mono / Stereo / 8-Stem DAW Session");
    println!("================================================================ algorithm\n");

    let corpora = vec![
        ("Sine Sweep", generate_sine_sweep(length, sample_rate)),
        ("Drum Transients", generate_drum_transient(length, sample_rate)),
        ("White Noise", generate_white_noise(length)),
        ("Silence", generate_silence(length)),
    ];

    println!("--- 1. SINGLE-TRACK COMPRESSION RATIO & THROUGHPUT EVALUATION ---");
    println!("{:<18} | {:<8} | {:<12} | {:<8} | {:<14} | {:<14}", "Signal Type", "Level", "Comp Bytes", "Ratio %", "Enc Throughput", "Dec Throughput");
    println!("--------------------------------------------------------------------------------------------------");

    for (sig_name, signal) in &corpora {
        let channels = vec![signal.clone()];
        for &level in &[0u8, 2u8, 5u8, 8u8] {
            let config = EncoderConfig::default_with_level(level, sample_rate, 16);

            let t0 = Instant::now();
            let mut iters = 0;
            let mut compressed = Vec::new();
            while t0.elapsed().as_millis() < 80 || iters < 2 {
                compressed = encode_track_with_config(&channels, "bench_track", &config).unwrap();
                iters += 1;
            }
            let enc_dur = t0.elapsed() / iters;

            let t1 = Instant::now();
            let mut dec_iters = 0;
            while t1.elapsed().as_millis() < 80 || dec_iters < 2 {
                let _ = decode_track(&compressed).unwrap();
                dec_iters += 1;
            }
            let dec_dur = t1.elapsed() / dec_iters;

            let ratio = (compressed.len() as f64 / raw_bytes as f64) * 100.0;
            let enc_mbs = (raw_bytes as f64 / (1024.0 * 1024.0)) / enc_dur.as_secs_f64();
            let dec_mbs = (raw_bytes as f64 / (1024.0 * 1024.0)) / dec_dur.as_secs_f64();

            println!("{:<18} | {:<8} | {:<12} | {:<7.2}% | {:<11.2} MB/s | {:<11.2} MB/s",
                     sig_name, format!("Loom-{}", level), compressed.len(), ratio, enc_mbs, dec_mbs);
        }
    }

    println!("\n--- 2. LOOM VS NATIVE FLAC COMPARISON (SINE SWEEP) ---");
    println!("{:<18} | {:<12} | {:<8} | {:<14}", "Codec / Level", "Comp Bytes", "Ratio %", "Enc Throughput");
    println!("----------------------------------------------------------------------------------");
    let signal = &corpora[0].1;
    let samples_i32: Vec<i32> = signal.iter().map(|&x| x as i32).collect();

    let t0 = Instant::now();
    let flac_bytes = encode_flac(&samples_i32, sample_rate, 4096);
    let flac_dur = t0.elapsed();
    let flac_ratio = (flac_bytes.len() as f64 / raw_bytes as f64) * 100.0;
    let flac_mbs = (raw_bytes as f64 / (1024.0 * 1024.0)) / flac_dur.as_secs_f64();
    println!("{:<18} | {:<12} | {:<7.2}% | {:<11.2} MB/s", "FLAC (Default)", flac_bytes.len(), flac_ratio, flac_mbs);

    for &level in &[0u8, 2u8, 5u8, 8u8] {
        let config = EncoderConfig::default_with_level(level, sample_rate, 16);
        let channels = vec![signal.clone()];
        let t0 = Instant::now();
        let loom_bytes = encode_track_with_config(&channels, "sine_sweep", &config).unwrap();
        let loom_dur = t0.elapsed();
        let loom_ratio = (loom_bytes.len() as f64 / raw_bytes as f64) * 100.0;
        let loom_mbs = (raw_bytes as f64 / (1024.0 * 1024.0)) / loom_dur.as_secs_f64();
        println!("{:<18} | {:<12} | {:<7.2}% | {:<11.2} MB/s", format!("Loom (Level {})", level), loom_bytes.len(), loom_ratio, loom_mbs);
    }

    println!("\n--- 3. MULTITRACK SESSION CONTAINER VS SEPARATE FLAC FILES (8 STEMS) ---");
    let stem_count = 8;
    let total_raw_bytes = raw_bytes * stem_count;
    let mut stems = Vec::new();
    let mut stem_names = Vec::new();

    for i in 0..stem_count {
        let name = format!("stem_{}", i);
        let data = vec![generate_sine_sweep(length, sample_rate)];
        stems.push(data);
        stem_names.push(name);
    }

    let indep_flac_bytes: usize = stems.iter().map(|s| {
        let i32_s: Vec<i32> = s[0].iter().map(|&x| x as i32).collect();
        encode_flac(&i32_s, sample_rate, 4096).len()
    }).sum();

    let session_loom_bytes = encode_session(&stems, &stem_names, sample_rate, 16, 4096, None, None, None).unwrap().len();

    let flac_multitrack_ratio = (indep_flac_bytes as f64 / total_raw_bytes as f64) * 100.0;
    let loom_session_ratio = (session_loom_bytes as f64 / total_raw_bytes as f64) * 100.0;
    let savings = ((indep_flac_bytes as f64 - session_loom_bytes as f64) / indep_flac_bytes as f64) * 100.0;

    println!("Total Raw PCM Size     : {} bytes ({:.2} MB)", total_raw_bytes, total_raw_bytes as f64 / (1024.0 * 1024.0));
    println!("8 Separate FLAC Files  : {} bytes ({:.2}% ratio)", indep_flac_bytes, flac_multitrack_ratio);
    println!("1 Loom Session (.loom) : {} bytes ({:.2}% ratio)", session_loom_bytes, loom_session_ratio);
    println!("Loom Session Storage Reduction over FLAC : {:.2}%", savings);

    println!("\n--- 4. RANDOM ACCESS SEEK LATENCY (SUB-MILLISECOND BOUNDS) ---");
    let config = EncoderConfig::default_with_level(5, sample_rate, 16);
    let channels = vec![corpora[0].1.clone()];
    let compressed = encode_track_with_config(&channels, "seek_track", &config).unwrap();

    let start_sample = (sample_rate * 2) as u64;
    let end_sample = (sample_rate * 3) as u64;
    let limit_samples = (end_sample - start_sample) as usize;

    let t0 = Instant::now();
    let mut seek_iters = 0;
    while t0.elapsed().as_millis() < 50 || seek_iters < 1000 {
        let _ = decode_track_partial(&compressed, 0, start_sample, limit_samples).unwrap();
        seek_iters += 1;
    }
    let seek_lat = t0.elapsed() / seek_iters;
    println!("Range Extraction (1 sec slice from 5 sec track) Latency: {:.3} us", seek_lat.as_nanos() as f64 / 1000.0);
    println!("================================================================================\n");
}
