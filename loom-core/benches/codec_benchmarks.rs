use criterion::{black_box, criterion_group, criterion_main, Criterion};
use loom_core::config::EncoderConfig;
use loom_core::{decode_track, encode_track_with_config};

fn generate_test_signal(num_samples: usize) -> Vec<i64> {
    (0..num_samples)
        .map(|i| {
            let t = i as f64 / 44100.0;
            let sig1 = (2.0 * std::f64::consts::PI * 440.0 * t).sin();
            let sig2 = (2.0 * std::f64::consts::PI * 880.0 * t).sin() * 0.5;
            let combined = (sig1 + sig2) * 16000.0;
            combined as i64
        })
        .collect()
}

fn bench_encode_decode(c: &mut Criterion) {
    let samples = vec![generate_test_signal(44100 * 2)];
    let config = EncoderConfig::new(5, 4096, 44100, 16);

    c.bench_function("encode_track_fast", |b| {
        b.iter(|| encode_track_with_config(black_box(&samples), "bench_stem", &config).unwrap())
    });

    let compressed = encode_track_with_config(&samples, "bench_stem", &config).unwrap();

    c.bench_function("decode_track_fast", |b| {
        b.iter(|| decode_track(black_box(&compressed)).unwrap())
    });
}

criterion_group!(benches, bench_encode_decode);
criterion_main!(benches);
