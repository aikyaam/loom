use crate::container::header::SessionHeader;
use std::io;

pub fn decode_track(session_bytes: &[u8]) -> io::Result<(Vec<Vec<i64>>, SessionHeader)> {
    let (mut tracks, header, _, _, _) =
        crate::container::session::decode_session_full(session_bytes)?;
    if tracks.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "No tracks found in session container",
        ));
    }
    Ok((std::mem::take(&mut tracks[0]), header))
}

pub fn decode_track_partial(
    session_bytes: &[u8],
    track_idx: usize,
    start_sample: u64,
    limit_samples: usize,
) -> io::Result<(Vec<Vec<i64>>, SessionHeader)> {
    let (mut tracks, header, _, _, _) =
        crate::container::session::decode_session_full(session_bytes)?;
    if track_idx >= tracks.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Track {} out of bounds", track_idx),
        ));
    }

    let track = &mut tracks[track_idx];
    if track.is_empty() {
        return Ok((track.clone(), header));
    }

    if start_sample as usize >= track[0].len() {
        let empty = vec![vec![]; track.len()];
        return Ok((empty, header));
    }

    let end_sample = std::cmp::min(start_sample as usize + limit_samples, track[0].len());
    let mut sliced = Vec::with_capacity(track.len());
    for ch in track.iter_mut() {
        sliced.push(ch[start_sample as usize..end_sample].to_vec());
    }
    Ok((sliced, header))
}
