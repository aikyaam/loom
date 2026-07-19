use criterion::{black_box, criterion_group, criterion_main, Criterion};
use flacenc::component::BitRepr;
use flacenc::error::Verify;
use loom_core::{decode_track, EncoderConfig, encode_track_with_config};

fn generate_sine_sweep(length: usize, sample_rate: u32) -> Vec<i64> {
    let mut data = Vec::with_capacity(length);
    for i in 0..length {
        let t = i as f64 / sample_rate as f64;
        let freq = 100.0 + (1000.0 - 100.0) * (i as f64 / length as f64);
        let val = (16383.0 * (2.0 * std::f64::consts::PI * freq * t).sin()).round() as i64;
        data.push(val);
    }
    data
}

fn generate_white_noise(length: usize) -> Vec<i64> {
    let mut data = Vec::with_capacity(length);
    for _ in 0..length {
        data.push(rand_s16());
    }
    data
}

fn generate_silence(length: usize) -> Vec<i64> {
    vec![0i64; length]
}

fn rand_s16() -> i64 {
    (fastrand::i16(..)) as i64
}

fn encode_loom(channels: &[Vec<i64>], level: u8, sample_rate: u32, name: &str) -> Vec<u8> {
    let config = EncoderConfig::default_with_level(level, sample_rate, 16);
    encode_track_with_config(channels, name, &config).unwrap()
}

fn encode_flac(samples: &[i32], sample_rate: u32, block_size: usize) -> Vec<u8> {
    let config = flacenc::config::Encoder::default().into_verified().unwrap();
    let source = flacenc::source::MemSource::from_samples(samples, 1, 16, sample_rate as usize);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, block_size).unwrap();
    let mut sink = flacenc::bitsink::MemSink::default();
    stream.write(&mut sink).unwrap();
    sink.as_slice().to_vec()
}

fn bench_encode_loom(c: &mut Criterion) {
    let sample_rate = 44100;
    let length = 44100 * 2;
    let mono = generate_sine_sweep(length, sample_rate);
    let stereo = vec![mono.clone(), mono.clone()];

    let mut group = c.benchmark_group("encode_loom");
    for level in 0..=8 {
        group.bench_function(format!("level_{}", level), |b| {
            b.iter(|| {
                let data = encode_loom(black_box(&stereo), level, sample_rate, "sine_sweep");
                black_box(data);
            })
        });
    }
    group.finish();
}

fn bench_decode_loom(c: &mut Criterion) {
    let sample_rate = 44100;
    let length = 44100 * 2;
    let mono = generate_sine_sweep(length, sample_rate);
    let stereo = vec![mono.clone(), mono.clone()];

    let mut group = c.benchmark_group("decode_loom");
    for level in [0, 5, 8] {
        let compressed = encode_loom(&stereo, level, sample_rate, "sine_sweep");
        group.bench_function(format!("level_{}", level), |b| {
            b.iter(|| {
                let (tracks, _) = decode_track(black_box(&compressed)).unwrap();
                black_box(tracks);
            })
        });
    }
    group.finish();
}

fn bench_loom_vs_flac(c: &mut Criterion) {
    let sample_rate = 44100;
    let length = 44100 * 2;
    let mono = generate_sine_sweep(length, sample_rate);
    let channels = vec![mono.clone()];
    let samples_i32: Vec<i32> = mono.iter().map(|&x| x as i32).collect();

    let mut group = c.benchmark_group("loom_vs_flac");
    group.bench_function("loom_level_5", |b| {
        b.iter(|| {
            let data = encode_loom(black_box(&channels), 5, sample_rate, "sine_sweep");
            black_box(data);
        })
    });
    group.bench_function("flac_default", |b| {
        b.iter(|| {
            let data = encode_flac(black_box(&samples_i32), sample_rate, 4096);
            black_box(data);
        })
    });
    group.finish();
}

fn bench_compression_ratio(c: &mut Criterion) {
    let sample_rate = 44100;
    let length = 44100 * 2;
    let raw_size = length * 2;

    let signals: Vec<(&str, Vec<i64>)> = vec![
        ("sine_sweep", generate_sine_sweep(length, sample_rate)),
        ("white_noise", generate_white_noise(length)),
        ("silence", generate_silence(length)),
    ];

    let mut group = c.benchmark_group("compression_ratio");
    for (name, signal) in &signals {
        let channels = vec![signal.clone()];
        for level in [0, 5, 8] {
            let compressed = encode_loom(&channels, level, sample_rate, name);
            group.bench_function(format!("{}_{}", name, level), |b| {
                b.iter(|| {
                    let data = encode_loom(black_box(&channels), level, sample_rate, name);
                    black_box(data);
                })
            });
            group.throughput(criterion::Throughput::Bytes(compressed.len() as u64));
        }
    }
    group.finish();

    // Print compression ratios
    println!("\n=== Compression Ratios (raw={} bytes) ===", raw_size);
    for (name, signal) in &signals {
        let channels = vec![signal.clone()];
        for level in [0, 5, 8] {
            let compressed = encode_loom(&channels, level, sample_rate, name);
            let ratio = compressed.len() as f64 / raw_size as f64;
            println!("  {:<12} level {}: {:>8} bytes ({:.1}%)", name, level, compressed.len(), ratio * 100.0);
        }
    }

    // FLAC comparison
    for (name, signal) in &signals {
        let samples_i32: Vec<i32> = signal.iter().map(|&x| x as i32).collect();
        let compressed = encode_flac(&samples_i32, sample_rate, 4096);
        let ratio = compressed.len() as f64 / raw_size as f64;
        println!("  {:<12} flac    : {:>8} bytes ({:.1}%)", name, compressed.len(), ratio * 100.0);
    }
    println!();
}

criterion_group!(
    benches,
    bench_encode_loom,
    bench_decode_loom,
    bench_loom_vs_flac,
    bench_compression_ratio,
);
criterion_main!(benches);
