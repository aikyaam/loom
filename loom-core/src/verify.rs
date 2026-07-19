use md5::{Digest, Md5};

pub fn compute_pcm_md5(interleaved_samples: &[i64], bit_depth: u8) -> [u8; 16] {
    let mut hasher = Md5::new();

    match bit_depth {
        16 => {
            for &sample in interleaved_samples {
                let val = sample as i16;
                hasher.update(&val.to_le_bytes());
            }
        }
        24 => {
            for &sample in interleaved_samples {
                let val = sample as i32;
                let bytes = val.to_le_bytes();

                hasher.update(&bytes[0..3]);
            }
        }
        32 => {
            for &sample in interleaved_samples {
                let val = sample as i32;
                hasher.update(&val.to_le_bytes());
            }
        }
        _ => {
            for &sample in interleaved_samples {
                hasher.update(&sample.to_le_bytes());
            }
        }
    }

    let result = hasher.finalize();
    let mut md5 = [0u8; 16];
    md5.copy_from_slice(&result);
    md5
}

pub fn verify_stream(computed: &[u8; 16], stored: &[u8; 16]) -> bool {
    computed == stored
}

pub fn compute_wasted_bits(samples: &[i64]) -> u8 {
    if samples.is_empty() {
        return 0;
    }
    let mut bits = 64;
    for &x in samples {
        if x != 0 {
            bits = std::cmp::min(bits, x.trailing_zeros());
        }
    }
    if bits == 64 {
        0
    } else {
        bits as u8
    }
}
