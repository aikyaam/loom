use criterion::{black_box, criterion_group, criterion_main, Criterion};
use flacenc::component::BitRepr;
use flacenc::error::Verify;
use loom_core::{decode_track, encode_track};

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

fn bench_codec(c: &mut Criterion) {
    let sample_rate = 44100;
    let length = 44100 * 2;
    let mono_signal = generate_sine_sweep(length, sample_rate);
    let channels = vec![mono_signal.clone()];

    c.bench_function("encode_track_loom", |b| {
        b.iter(|| {
            let compressed = encode_track(
                black_box(&channels),
                black_box(sample_rate),
                black_box(16),
                black_box(4096),
                black_box("sine_sweep"),
            )
            .unwrap();
            black_box(compressed);
        })
    });

    let loom_compressed = encode_track(&channels, sample_rate, 16, 4096, "sine_sweep").unwrap();

    c.bench_function("decode_track_loom", |b| {
        b.iter(|| {
            let decompressed = decode_track(black_box(&loom_compressed)).unwrap();
            black_box(decompressed);
        })
    });

    let samples_i32: Vec<i32> = mono_signal.iter().map(|&x| x as i32).collect();

    c.bench_function("encode_track_flac", |b| {
        b.iter(|| {
            let config = flacenc::config::Encoder::default().into_verified().unwrap();
            let source =
                flacenc::source::MemSource::from_samples(&samples_i32, 1, 16, sample_rate as usize);
            let stream = flacenc::encode_with_fixed_block_size(&config, source, 4096).unwrap();
            black_box(stream);
        })
    });

    let config = flacenc::config::Encoder::default().into_verified().unwrap();
    let source =
        flacenc::source::MemSource::from_samples(&samples_i32, 1, 16, sample_rate as usize);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, 4096).unwrap();

    let mut sink = flacenc::bitsink::MemSink::default();
    stream.write(&mut sink).unwrap();
    let flac_bytes = sink.as_slice().to_vec();

    c.bench_function("decode_track_flac", |b| {
        b.iter(|| {
            let cursor = std::io::Cursor::new(black_box(&flac_bytes));
            let mut reader = claxon::FlacReader::new(cursor).unwrap();
            let mut samples = Vec::with_capacity(length);
            for sample in reader.samples() {
                samples.push(sample.unwrap());
            }
            black_box(samples);
        })
    });
}

criterion_group!(benches, bench_codec);
criterion_main!(benches);
