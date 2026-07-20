use loom_core::config::EncoderConfig;
use loom_core::{decode_session, encode_session_with_config};

#[test]
fn test_cross_track_prediction_roundtrip() {
    let sample_rate = 44100;
    let bit_depth = 16;
    let num_samples = 44100;

    let track0_channel: Vec<i64> = (0..num_samples)
        .map(|i| ((i as f64 * 0.05).sin() * 12000.0) as i64)
        .collect();

    let track1_channel: Vec<i64> = track0_channel
        .iter()
        .map(|&x| (x * 3 / 4) + ((x as f64 * 0.001).cos() * 5.0) as i64)
        .collect();

    let tracks = vec![vec![track0_channel.clone()], vec![track1_channel.clone()]];
    let track_names = vec!["guitar".to_string(), "bass".to_string()];

    let config = EncoderConfig::new(5, 4096, sample_rate, bit_depth);

    let compressed = encode_session_with_config(&tracks, &track_names, &config, None, None, None)
        .expect("Encoding cross-track session failed");

    let (decoded_tracks, _header) =
        decode_session(&compressed).expect("Decoding cross-track session failed");

    assert_eq!(decoded_tracks.len(), 2);
    assert_eq!(decoded_tracks[0][0], track0_channel);
    assert_eq!(decoded_tracks[1][0], track1_channel);
}
