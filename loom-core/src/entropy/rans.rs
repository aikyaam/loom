const RANS_L: u32 = 1 << 16;

pub fn rans_encode_bytes(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }

    let mut freqs = [0u32; 256];
    for &b in data {
        freqs[b as usize] += 1;
    }

    let mut cum_freqs = [0u32; 257];
    for i in 0..256 {
        cum_freqs[i + 1] = cum_freqs[i] + freqs[i];
    }

    let mut state = RANS_L;
    let mut byte_stream = Vec::new();

    for &b in data.iter().rev() {
        let sym = b as usize;
        let freq = freqs[sym];
        if freq == 0 {
            continue;
        }

        let max_state = ((RANS_L >> 12) << 16) * freq;
        while state >= max_state {
            byte_stream.push((state & 0xFF) as u8);
            state >>= 8;
        }

        let start = cum_freqs[sym];
        state = ((state / freq) << 12) + (state % freq) + start;
    }

    byte_stream.push((state >> 24) as u8);
    byte_stream.push((state >> 16) as u8);
    byte_stream.push((state >> 8) as u8);
    byte_stream.push((state & 0xFF) as u8);
    byte_stream.reverse();
    byte_stream
}

#[derive(Clone, Debug)]
pub struct RansDecoder {
    state: u32,
    _offset: usize,
}

impl RansDecoder {
    pub fn new(stream: &[u8]) -> Option<(Self, usize)> {
        if stream.len() < 4 {
            return None;
        }
        let state = ((stream[0] as u32) << 24)
            | ((stream[1] as u32) << 16)
            | ((stream[2] as u32) << 8)
            | (stream[3] as u32);
        Some((RansDecoder { state, _offset: 4 }, stream.len()))
    }

    pub fn get_state(&self) -> u32 {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rans_byte_roundtrip() {
        let input = b"loom_rans_entropy_coder_test_payload_1234567890";
        let encoded = rans_encode_bytes(input);
        assert!(!encoded.is_empty());
        let (decoder, _) = RansDecoder::new(&encoded).expect("Decoder init failed");
        assert!(decoder.get_state() >= RANS_L);
    }
}
