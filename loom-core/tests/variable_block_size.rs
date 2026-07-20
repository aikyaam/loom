use loom_core::analyze::detect_transient;
use loom_core::decoder::decode_track;
use loom_core::encoder::encode_track;

#[test]
fn test_variable_block_size_transient() {
    let total_samples = 4000;
    let mut channel = vec![0i64; total_samples];

    for i in 0..1500 {
        channel[i] = (i % 2) as i64;
    }

    for i in 1500..2000 {
        channel[i] = 10000;
    }

    for i in 2000..4000 {
        channel[i] = (i % 2) as i64;
    }

    let pcm = vec![channel.clone()];

    let compressed =
        encode_track(&pcm, 44100, 16, 1000, "transient_track").expect("Encoding failed");

    let (decoded, _header) = decode_track(&compressed).expect("Decoding failed");

    assert_eq!(decoded[0].len(), total_samples);
    assert_eq!(decoded[0], channel);

    let transient_idx = detect_transient(&channel[1000..2000]);
    assert!(transient_idx.is_some(), "Transient should be detected");
    let idx = transient_idx.unwrap();
    assert!(
        idx >= 384 && idx <= 500,
        "Transient detected at window start {}",
        idx
    );
}
