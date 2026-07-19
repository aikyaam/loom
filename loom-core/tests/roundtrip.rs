use loom_core::decoder::decode_track;
use loom_core::encoder::encode_track;
use loom_core::verify::{compute_pcm_md5, verify_stream};

fn test_signal_roundtrip(
    pcm: Vec<Vec<i64>>,
    sample_rate: u32,
    bit_depth: u8,
    block_size: u32,
    name: &str,
) {
    let compressed =
        encode_track(&pcm, sample_rate, bit_depth, block_size, name).expect("Encoding failed");

    let (decoded, header) = decode_track(&compressed).expect("Decoding failed");

    assert_eq!(header.sample_rate, sample_rate);
    assert_eq!(header.bit_depth, bit_depth);
    assert_eq!(header.tracks.len(), 1);
    assert_eq!(header.tracks[0].name, name);

    assert_eq!(decoded.len(), pcm.len(), "Channel count mismatch");
    for ch in 0..pcm.len() {
        assert_eq!(
            decoded[ch].len(),
            pcm[ch].len(),
            "Sample count mismatch on channel {}",
            ch
        );
        assert_eq!(
            decoded[ch], pcm[ch],
            "Decoded samples differ on channel {}",
            ch
        );
    }

    let total_samples = pcm[0].len();
    let mut interleaved = Vec::with_capacity(total_samples * pcm.len());
    for s in 0..total_samples {
        for ch in 0..pcm.len() {
            interleaved.push(pcm[ch][s]);
        }
    }
    let computed = compute_pcm_md5(&interleaved, bit_depth);
    assert!(
        verify_stream(&computed, &header.tracks[0].md5),
        "MD5 mismatch!"
    );
}

#[test]
fn test_silence() {
    let pcm = vec![vec![0i64; 1000]];
    test_signal_roundtrip(pcm, 44100, 16, 256, "silence_mono");

    let pcm2 = vec![vec![0i64; 1000], vec![0i64; 1000]];
    test_signal_roundtrip(pcm2, 44100, 16, 256, "silence_stereo");
}

#[test]
fn test_constant() {
    let pcm = vec![vec![12345i64; 1000]];
    test_signal_roundtrip(pcm, 44100, 16, 256, "dc_level");
}

#[test]
fn test_sine_sweep() {
    let length = 4000;
    let mut channel = vec![0i64; length];
    for i in 0..length {
        let t = i as f64 / 44100.0;
        let sample = (32767.0 * (2.0 * std::f64::consts::PI * 440.0 * t).sin()).round() as i64;
        channel[i] = sample;
    }
    test_signal_roundtrip(vec![channel], 44100, 16, 512, "sine_mono");
}

#[test]
fn test_sine_sweep_stereo() {
    let length = 4000;
    let mut left = vec![0i64; length];
    let mut right = vec![0i64; length];
    for i in 0..length {
        let t = i as f64 / 48000.0;
        left[i] = (8388607.0 * (2.0 * std::f64::consts::PI * 440.0 * t).sin()).round() as i64;
        right[i] = (8388607.0 * (2.0 * std::f64::consts::PI * 880.0 * t).sin()).round() as i64;
    }
    test_signal_roundtrip(vec![left, right], 48000, 24, 1024, "sine_stereo_24bit");
}

#[test]
fn test_noise() {
    let length = 2000;
    let mut left = vec![0i64; length];
    let mut right = vec![0i64; length];

    let mut seed = 123456789u64;
    for i in 0..length {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let val_l = (seed as i16) as i64;
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let val_r = (seed as i16) as i64;
        left[i] = val_l;
        right[i] = val_r;
    }
    test_signal_roundtrip(vec![left, right], 44100, 16, 256, "noise_stereo");
}

#[test]
fn test_session_roundtrip() {
    use loom_core::{decode_session, encode_session};

    let length = 4000;

    let mut track0 = vec![vec![0i64; length]];
    for i in 0..length {
        let t = i as f64 / 44100.0;
        track0[0][i] = (16383.0 * (2.0 * std::f64::consts::PI * 440.0 * t).sin()).round() as i64;
    }

    let mut track1 = vec![vec![0i64; length]];
    for i in 0..length {
        track1[0][i] = (track0[0][i] * 3) / 4 + (i % 10) as i64;
    }

    let tracks = vec![track0, track1];
    let names = vec!["drums".to_string(), "synth".to_string()];

    let compressed =
        encode_session(&tracks, &names, 44100, 16, 512, None, None, None).expect("Session encoding failed");

    let (decoded, header) = decode_session(&compressed).expect("Session decoding failed");

    assert_eq!(header.sample_rate, 44100);
    assert_eq!(header.bit_depth, 16);
    assert_eq!(header.tracks.len(), 2);
    assert_eq!(header.tracks[0].name, "drums");
    assert_eq!(header.tracks[1].name, "synth");

    assert_eq!(decoded.len(), 2);
    for t in 0..2 {
        assert_eq!(decoded[t].len(), tracks[t].len());
        for ch in 0..tracks[t].len() {
            assert_eq!(
                decoded[t][ch], tracks[t][ch],
                "Track {} channel {} mismatch",
                t, ch
            );
        }
    }
}

#[test]
fn test_range_extraction() {
    use loom_core::{decode_track_partial, encode_session};

    let length = 4000;

    let mut track0 = vec![vec![0i64; length]];
    for i in 0..length {
        let t = i as f64 / 44100.0;
        track0[0][i] = (16383.0 * (2.0 * std::f64::consts::PI * 440.0 * t).sin()).round() as i64;
    }

    let mut track1 = vec![vec![0i64; length]];
    for i in 0..length {
        track1[0][i] = (track0[0][i] * 3) / 4 + (i % 10) as i64;
    }

    let tracks = vec![track0.clone(), track1.clone()];
    let names = vec!["drums".to_string(), "synth".to_string()];

    let compressed =
        encode_session(&tracks, &names, 44100, 16, 512, None, None, None).expect("Session encoding failed");

    let (sliced0, _) =
        decode_track_partial(&compressed, 0, 300, 1200).expect("Range decoding failed on Track 0");
    assert_eq!(sliced0[0], track0[0][300..1500]); // 300.. (300+1200)

    let (sliced1, _) =
        decode_track_partial(&compressed, 1, 600, 2500).expect("Range decoding failed on Track 1");
    assert_eq!(sliced1[0], track1[0][600..3100]); // 600.. (600+2500)
}

#[test]
fn test_non_destructive_edits() {
    use loom_core::{
        decode_session, decode_session_full, encode_session, EditBlock, Fade, FadeShape,
        MuteRegion, TrackEdits,
    };
    use std::collections::HashMap;

    let length = 1000;
    let track0 = vec![vec![1000i64; length]];
    let tracks = vec![track0];
    let names = vec!["constant".to_string()];

    let compressed_v1 =
        encode_session(&tracks, &names, 44100, 16, 256, None, None, None).expect("Session encoding failed");

    let mut track_edits = TrackEdits::new();
    track_edits.mutes.push(MuteRegion {
        start_sample: 100,
        end_sample: 200,
    });
    track_edits.fades.push(Fade {
        start_sample: 400,
        end_sample: 900,
        shape: FadeShape::Linear,
        is_fade_in: false,
    });

    let mut tracks_edits = HashMap::new();
    tracks_edits.insert(0u16, track_edits);
    let edit_block = EditBlock { tracks_edits };

    let (decoded_tracks, header, _, tags, picture) = decode_session_full(&compressed_v1).expect("Decoding failed");
    let compressed_v2 = encode_session(
        &decoded_tracks,
        &names,
        header.sample_rate,
        header.bit_depth,
        256,
        Some(&edit_block),
        tags.as_ref(),
        picture.as_ref(),
    ).expect("Updating edits failed");

    let (decoded, _) = decode_session(&compressed_v2).expect("Decoding modified session failed");

    println!("Decoded tracks count: {}", decoded.len());
    println!("Decoded track 0 samples count: {}", decoded[0][0].len());
    println!("Decoded track 0 sample 100: {}", decoded[0][0][100]);
    println!("Decoded track 0 sample 400: {}", decoded[0][0][400]);

    let pcm = &decoded[0][0];

    for i in 100..200 {
        assert_eq!(pcm[i], 0, "Mute failed at index {}", i);
    }

    for i in 0..100 {
        assert_eq!(pcm[i], 1000);
    }
    for i in 200..400 {
        assert_eq!(pcm[i], 1000);
    }

    assert_eq!(pcm[400], 1000);
    assert!(pcm[899] <= 2);
    assert_eq!(pcm[900], 1000);
    for i in 401..900 {
        let expected = (1000.0 * (1.0 - (i - 400) as f32 / 500.0)).round() as i64;
        assert!(
            (pcm[i] - expected).abs() <= 1,
            "Fade value mismatch at index {}: got {}, expected {}",
            i,
            pcm[i],
            expected
        );
    }
}

#[test]
fn test_version_diffing() {
    use loom_core::{apply_diff, encode_diff, encode_session};

    let length = 4000;

    let track_v1 = vec![1000i64; length];
    let tracks_v1 = vec![vec![track_v1]];
    let names = vec!["drums".to_string()];

    let mut track_v2 = vec![1000i64; length];
    for i in 512..1024 {
        track_v2[i] = 2000;
    }
    let tracks_v2 = vec![vec![track_v2]];

    let compressed_v1 =
        encode_session(&tracks_v1, &names, 44100, 16, 256, None, None, None).expect("V1 encoding failed");

    let compressed_v2 =
        encode_session(&tracks_v2, &names, 44100, 16, 256, None, None, None).expect("V2 encoding failed");

    let diff = encode_diff(&compressed_v1, &compressed_v2).expect("Diff encoding failed");

    let mut num_copies = 0;
    let mut num_inserts = 0;
    for td in &diff.tracks_diffs {
        for inst in &td.instructions {
            match inst {
                loom_core::FrameInstruction::Copy { .. } => num_copies += 1,
                loom_core::FrameInstruction::Insert { .. } => num_inserts += 1,
            }
        }
    }
    assert_eq!(num_inserts, 2);
    assert_eq!(num_copies, 14);

    let reconstructed_v2 = apply_diff(&compressed_v1, &diff).expect("Applying diff failed");

    assert_eq!(
        reconstructed_v2, compressed_v2,
        "Reconstructed stream mismatch"
    );
}
#[test]
fn test_sync_recovery() {
    use loom_core::{decode_session_full, encode_session, Frame};

    let length = 1024;
    let track0 = vec![1000i64; length];
    let track1 = vec![1000i64; length];
    let tracks = vec![vec![track0], vec![track1]];
    let names = vec!["drums".to_string(), "synth".to_string()];

    let compressed = encode_session(&tracks, &names, 44100, 16, 256, None, None, None).expect("Encoding failed");

    let mut v2_bytes = compressed.clone();

    let sync0 = Frame::scan_for_sync(&v2_bytes, 0).expect("Sync 0 not found");

    let sync1 = Frame::scan_for_sync(&v2_bytes, sync0 + 2).expect("Sync 1 not found");

    v2_bytes[sync1] = 0x00;
    v2_bytes[sync1 + 1] = 0x00;

    let (decoded, _, _, _, _) =
        decode_session_full(&v2_bytes).expect("Decoding failed despite corruption");

    assert!(!decoded[0][0].is_empty());
}

#[test]
fn test_picture_block() {
    use loom_core::{
        decode_session_full, encode_session, PictureBlock, PictureType,
    };

    let length = 512;
    let track = vec![1000i64; length];
    let tracks = vec![vec![track]];
    let names = vec!["drums".to_string()];

    let compressed = encode_session(&tracks, &names, 44100, 16, 256, None, None, None).expect("Encoding failed");

    let original_picture = PictureBlock {
        picture_type: PictureType::FrontCover,
        mime_type: "image/png".to_string(),
        description: "Front Album Cover".to_string(),
        width: 300,
        height: 300,
        color_depth: 24,
        num_colors: 0,
        data: vec![0xCA, 0xFE, 0xBA, 0xBE],
    };

    let (decoded_tracks, header, edits, tags, _) = decode_session_full(&compressed).expect("Decoding failed");
    let updated = encode_session(
        &decoded_tracks,
        &names,
        header.sample_rate,
        header.bit_depth,
        256,
        edits.as_ref(),
        tags.as_ref(),
        Some(&original_picture),
    ).expect("Failed to append picture block");

    let (_, _, _, _, decoded_picture) =
        decode_session_full(&updated).expect("Failed to decode session containing picture block");

    assert!(decoded_picture.is_some());
    let pb = decoded_picture.unwrap();
    assert_eq!(pb.picture_type, PictureType::FrontCover);
    assert_eq!(pb.mime_type, "image/png");
    assert_eq!(pb.description, "Front Album Cover");
    assert_eq!(pb.width, 300);
    assert_eq!(pb.height, 300);
    assert_eq!(pb.data, vec![0xCA, 0xFE, 0xBA, 0xBE]);
}

#[test]
fn test_multichannel_roundtrip() {
    use loom_core::{decode_track, encode_track};

    let length = 512;

    let mut channels = Vec::new();
    for ch in 0..6 {
        let mut data = Vec::with_capacity(length);
        for i in 0..length {
            let val = (1000.0 * ((i as f64 * (ch + 1) as f64) / 100.0).sin()).round() as i64;
            data.push(val);
        }
        channels.push(data);
    }

    let compressed = encode_track(&channels, 48000, 16, 256, "surround_5_1")
        .expect("Failed to encode 6-channel audio");

    let (decoded, header) = decode_track(&compressed).expect("Failed to decode 6-channel audio");

    assert_eq!(header.bit_depth, 16);
    assert_eq!(header.sample_rate, 48000);
    assert_eq!(decoded.len(), 6);
    for ch in 0..6 {
        assert_eq!(decoded[ch].len(), length);
        assert_eq!(decoded[ch], channels[ch]);
    }
}

#[test]
fn test_wasted_bits() {
    use loom_core::{decode_track, encode_track};

    let length = 512;

    let mut channel = Vec::with_capacity(length);
    for i in 0..length {
        let val = (100.0 * (i as f64 / 10.0).sin()).round() as i64 * 16;
        channel.push(val);
    }
    let channels = vec![channel];

    let compressed = encode_track(&channels, 44100, 16, 256, "mono")
        .expect("Failed to encode track with wasted bits");

    let (decoded, header) =
        decode_track(&compressed).expect("Failed to decode track with wasted bits");

    assert_eq!(header.bit_depth, 16);
    assert_eq!(header.sample_rate, 44100);
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0], channels[0]);
}
