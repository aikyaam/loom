use crate::container::session::encode_session;
use std::io;

pub fn encode_track(
    pcm_channels: &[Vec<i64>],
    sample_rate: u32,
    bit_depth: u8,
    block_size: u32,
    track_name: &str,
) -> io::Result<Vec<u8>> {
    let tracks = vec![pcm_channels.to_vec()];
    let track_names = vec![track_name.to_string()];
    encode_session(
        &tracks,
        &track_names,
        sample_rate,
        bit_depth,
        block_size as usize,
        None,
        None,
        None,
    )
}
