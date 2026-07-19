use crate::{decode_session_full, encode_track};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;

#[no_mangle]
pub unsafe extern "C" fn loom_encode_track(
    samples: *const i32,
    total_samples: usize,
    num_channels: u32,
    sample_rate: u32,
    bit_depth: u32,
    block_size: u32,
    track_name: *const c_char,
    out_len: *mut usize,
) -> *mut u8 {
    if samples.is_null() || out_len.is_null() || num_channels == 0 || total_samples == 0 {
        return ptr::null_mut();
    }

    let c_str = if track_name.is_null() {
        "track"
    } else {
        match CStr::from_ptr(track_name).to_str() {
            Ok(s) => s,
            Err(_) => "track",
        }
    };

    let mut channels = vec![vec![0i64; total_samples]; num_channels as usize];
    let raw_slice = std::slice::from_raw_parts(samples, total_samples * num_channels as usize);
    for i in 0..(total_samples * num_channels as usize) {
        let ch = i % num_channels as usize;
        let idx = i / num_channels as usize;
        channels[ch][idx] = raw_slice[i] as i64;
    }

    match encode_track(&channels, sample_rate, bit_depth as u8, block_size, c_str) {
        Ok(compressed) => {
            let mut boxed_slice = compressed.into_boxed_slice();
            let ptr = boxed_slice.as_mut_ptr();
            *out_len = boxed_slice.len();
            std::mem::forget(boxed_slice);
            ptr
        }
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn loom_decode_track(
    compressed_data: *const u8,
    compressed_len: usize,
    out_channels: *mut u32,
    out_samples: *mut usize,
    out_sample_rate: *mut u32,
    out_bit_depth: *mut u32,
) -> *mut i32 {
    if compressed_data.is_null()
        || compressed_len == 0
        || out_channels.is_null()
        || out_samples.is_null()
        || out_sample_rate.is_null()
        || out_bit_depth.is_null()
    {
        return ptr::null_mut();
    }

    let raw_compressed = std::slice::from_raw_parts(compressed_data, compressed_len);
    match crate::decoder::decode_track_partial(raw_compressed, 0, 0, usize::MAX) {
        Ok((pcm_channels, header)) => {
            if pcm_channels.is_empty() {
                return ptr::null_mut();
            }

            let num_channels = pcm_channels.len();
            let total_samples = pcm_channels[0].len();

            *out_channels = num_channels as u32;
            *out_samples = total_samples;
            *out_sample_rate = header.sample_rate;
            *out_bit_depth = header.bit_depth as u32;

            let mut interleaved = Vec::with_capacity(total_samples * num_channels);
            for s in 0..total_samples {
                for ch in 0..num_channels {
                    interleaved.push(pcm_channels[ch][s] as i32);
                }
            }

            let mut boxed_slice = interleaved.into_boxed_slice();
            let ptr = boxed_slice.as_mut_ptr();
            std::mem::forget(boxed_slice);
            ptr
        }
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn loom_free_buffer(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, len));
    }
}

#[no_mangle]
pub unsafe extern "C" fn loom_free_samples(ptr: *mut i32, len: usize) {
    if !ptr.is_null() && len > 0 {
        let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, len));
    }
}

#[no_mangle]
pub unsafe extern "C" fn loom_decode_session_track(
    session_data: *const u8,
    session_len: usize,
    track_idx: u32,
    out_channels: *mut u32,
    out_samples: *mut usize,
) -> *mut i32 {
    if session_data.is_null() || session_len == 0 || out_channels.is_null() || out_samples.is_null()
    {
        return ptr::null_mut();
    }

    let raw_session = std::slice::from_raw_parts(session_data, session_len);
    match decode_session_full(raw_session) {
        Ok((tracks_pcm, _header, _, _, _)) => {
            let t = track_idx as usize;
            if t >= tracks_pcm.len() || tracks_pcm[t].is_empty() {
                return ptr::null_mut();
            }

            let pcm_channels = &tracks_pcm[t];
            let num_channels = pcm_channels.len();
            let total_samples = pcm_channels[0].len();

            *out_channels = num_channels as u32;
            *out_samples = total_samples;

            let mut interleaved = Vec::with_capacity(total_samples * num_channels);
            for s in 0..total_samples {
                for ch in 0..num_channels {
                    interleaved.push(pcm_channels[ch][s] as i32);
                }
            }

            let mut boxed_slice = interleaved.into_boxed_slice();
            let ptr = boxed_slice.as_mut_ptr();
            std::mem::forget(boxed_slice);
            ptr
        }
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn loom_alloc_buffer(size: usize) -> *mut u8 {
    let mut buf = vec![0u8; size];
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}
