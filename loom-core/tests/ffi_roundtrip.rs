use loom_core::ffi::{loom_decode_track, loom_encode_track, loom_free_buffer, loom_free_samples};

#[test]
fn test_ffi_roundtrip() {
    let total_samples = 512;
    let num_channels = 2;
    let mut original_samples = vec![0i32; total_samples * num_channels];
    for i in 0..total_samples {
        let t = i as f64 / 44100.0;
        let left_val = (1000.0 * (2.0 * std::f64::consts::PI * 440.0 * t).sin()).round() as i32;
        let right_val = (800.0 * (2.0 * std::f64::consts::PI * 880.0 * t).sin()).round() as i32;
        original_samples[i * 2] = left_val;
        original_samples[i * 2 + 1] = right_val;
    }

    unsafe {
        let mut compressed_len: usize = 0;
        let track_name = b"ffi_track\0".as_ptr() as *const std::os::raw::c_char;

        let compressed_ptr = loom_encode_track(
            original_samples.as_ptr(),
            total_samples,
            num_channels as u32,
            44100,
            16,
            256,
            track_name,
            &mut compressed_len,
        );

        assert!(!compressed_ptr.is_null());
        assert!(compressed_len > 0);

        let mut out_channels: u32 = 0;
        let mut out_samples: usize = 0;
        let mut out_sample_rate: u32 = 0;
        let mut out_bit_depth: u32 = 0;

        let decoded_ptr = loom_decode_track(
            compressed_ptr,
            compressed_len,
            &mut out_channels,
            &mut out_samples,
            &mut out_sample_rate,
            &mut out_bit_depth,
        );

        assert!(!decoded_ptr.is_null());
        assert_eq!(out_channels, num_channels as u32);
        assert_eq!(out_samples, total_samples);
        assert_eq!(out_sample_rate, 44100);
        assert_eq!(out_bit_depth, 16);

        let decoded_slice = std::slice::from_raw_parts(decoded_ptr, total_samples * num_channels);
        for i in 0..(total_samples * num_channels) {
            assert_eq!(decoded_slice[i], original_samples[i]);
        }

        loom_free_buffer(compressed_ptr, compressed_len);
        loom_free_samples(decoded_ptr, total_samples * num_channels);
    }
}
