use crate::config::EncoderConfig;
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
    let config = EncoderConfig::new(5, block_size as usize, sample_rate, bit_depth);
    crate::container::session::encode_session_with_config(
        &tracks,
        &track_names,
        &config,
        None,
        None,
        None,
    )
}

pub fn encode_track_with_config(
    pcm_channels: &[Vec<i64>],
    track_name: &str,
    config: &EncoderConfig,
) -> io::Result<Vec<u8>> {
    let tracks = vec![pcm_channels.to_vec()];
    let track_names = vec![track_name.to_string()];
    crate::container::session::encode_session_with_config(
        &tracks,
        &track_names,
        config,
        None,
        None,
        None,
    )
}
