use crate::container::header::SessionHeader;
use crate::diff::container::{FrameInstruction, SessionDiff, TrackDiff};
use md5::{Digest, Md5};
use std::io::{self, Cursor, Read};

pub fn extract_raw_frames(session_bytes: &[u8]) -> io::Result<Vec<Vec<Vec<u8>>>> {
    let mut cursor = Cursor::new(session_bytes);
    let header = SessionHeader::deserialize(&mut cursor)?;
    let num_tracks = header.tracks.len();

    let mut term_buf = [0u8; 1];
    loop {
        cursor.read_exact(&mut term_buf)?;
        if term_buf[0] == 0xFF {
            break;
        }
        let mut len_buf = [0u8; 4];
        cursor.read_exact(&mut len_buf)?;
        let length = u32::from_be_bytes(len_buf) as usize;
        let pos = cursor.position();
        cursor.set_position(pos + length as u64);
    }

    let mut track_frames = vec![Vec::new(); num_tracks];

    loop {
        let start_pos = cursor.position() as usize;
        let mut fl_buf = [0u8; 4];
        if cursor.read_exact(&mut fl_buf).is_err() {
            break;
        }
        let frame_len = u32::from_be_bytes(fl_buf) as usize;
        let mut frame_payload = vec![0u8; frame_len];
        cursor.read_exact(&mut frame_payload)?;

        let mut reader = crate::bitstream::BitReader::new(&frame_payload);
        let _sync = reader.read_bits(16)?;
        let track_idx = reader.read_bits(16)? as usize;

        let total_frame_len = 4 + frame_len;
        let raw_frame_bytes = session_bytes[start_pos..(start_pos + total_frame_len)].to_vec();

        if track_idx < num_tracks {
            track_frames[track_idx].push(raw_frame_bytes);
        }
    }

    Ok(track_frames)
}

pub fn encode_diff(v1_bytes: &[u8], v2_bytes: &[u8]) -> io::Result<SessionDiff> {
    let mut hasher = Md5::new();
    hasher.update(v1_bytes);
    let mut base_md5 = [0u8; 16];
    base_md5.copy_from_slice(&hasher.finalize());

    let mut cursor = Cursor::new(v2_bytes);
    let _header = SessionHeader::deserialize(&mut cursor)?;
    let mut term_buf = [0u8; 1];
    loop {
        cursor.read_exact(&mut term_buf)?;
        if term_buf[0] == 0xFF {
            break;
        }
        let mut len_buf = [0u8; 4];
        cursor.read_exact(&mut len_buf)?;
        let length = u32::from_be_bytes(len_buf) as usize;
        let pos = cursor.position();
        cursor.set_position(pos + length as u64);
    }
    let metadata_len = cursor.position() as usize;
    let metadata_payload = v2_bytes[0..metadata_len].to_vec();

    let v1_frames = extract_raw_frames(v1_bytes)?;
    let v2_frames = extract_raw_frames(v2_bytes)?;

    let num_tracks = v2_frames.len();
    let mut tracks_diffs = Vec::with_capacity(num_tracks);

    for t in 0..num_tracks {
        let mut instructions = Vec::new();
        let v2_track_frames = &v2_frames[t];
        let v1_track_frames = if t < v1_frames.len() {
            Some(&v1_frames[t])
        } else {
            None
        };

        for i in 0..v2_track_frames.len() {
            let frame_bytes = &v2_track_frames[i];

            let mut match_found = false;
            if let Some(v1_tf) = v1_track_frames {
                if i < v1_tf.len() && v1_tf[i] == *frame_bytes {
                    instructions.push(FrameInstruction::Copy {
                        base_frame_idx: i as u32,
                    });
                    match_found = true;
                }
            }

            if !match_found {
                let payload = frame_bytes[4..].to_vec();
                instructions.push(FrameInstruction::Insert {
                    frame_bytes: payload,
                });
            }
        }

        tracks_diffs.push(TrackDiff {
            track_idx: t as u16,
            instructions,
        });
    }

    Ok(SessionDiff {
        base_md5,
        metadata_payload,
        tracks_diffs,
    })
}
