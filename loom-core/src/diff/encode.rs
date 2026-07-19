use crate::container::header::SessionHeader;
use crate::container::seek_index::SeekTable;
use crate::container::edit_block::EditBlock;
use crate::diff::container::{FrameInstruction, SessionDiff, TrackDiff};
use md5::{Digest, Md5};
use std::io::{self, Cursor, Read};

fn extract_loom_payload_and_track0(session_bytes: &[u8]) -> io::Result<(Vec<u8>, SessionHeader, Vec<u8>)> {
    let mut cursor = Cursor::new(session_bytes);
    let mut magic = [0u8; 4];
    cursor.read_exact(&mut magic)?;

    let is_old_format = magic == *b"LSE\x01" || magic == *b"LOOM";
    if &magic != b"fLaC" && !is_old_format {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Not a FLAC or Loom file",
        ));
    }

    let mut loom_payload = Vec::new();
    if is_old_format {
        loom_payload.extend_from_slice(session_bytes);
    } else {
        loop {
            let mut header = [0u8; 4];
            cursor.read_exact(&mut header)?;
            let is_last = (header[0] & 0x80) != 0;
            let block_type = header[0] & 0x7F;
            let length = u32::from_be_bytes([0, header[1], header[2], header[3]]) as usize;

            if block_type == 2 {
                let mut app_id = [0u8; 4];
                cursor.read_exact(&mut app_id)?;
                if &app_id == b"LOOM" {
                    let mut data = vec![0u8; length - 4];
                    cursor.read_exact(&mut data)?;
                    loom_payload.extend_from_slice(&data);
                } else {
                    cursor.set_position(cursor.position() + length as u64 - 4);
                }
            } else {
                cursor.set_position(cursor.position() + length as u64);
            }

            if is_last {
                break;
            }
        }
    }

    let track0_frames_offset = cursor.position() as usize;
    let track0_payload = if is_old_format {
        Vec::new()
    } else {
        session_bytes[track0_frames_offset..].to_vec()
    };

    let mut loom_cursor = Cursor::new(&loom_payload);
    let header = SessionHeader::deserialize(&mut loom_cursor)?;
    Ok((loom_payload, header, track0_payload))
}

pub fn extract_raw_frames(session_bytes: &[u8]) -> io::Result<Vec<Vec<Vec<u8>>>> {
    let (loom_payload, header, track0_payload) = extract_loom_payload_and_track0(session_bytes)?;
    let num_tracks = header.tracks.len();

    let mut loom_cursor = Cursor::new(&loom_payload);
    let _header = SessionHeader::deserialize(&mut loom_cursor)?;
    let seek_table = SeekTable::deserialize(&mut loom_cursor)?;
    let _edit_block = EditBlock::deserialize(&mut loom_cursor)?;

    let loom_pos = loom_cursor.position() as usize;
    let loom_frames_payload = &loom_payload[loom_pos..];

    let mut track_frames = vec![Vec::new(); num_tracks];

    for t in 0..num_tracks {
        let points = &seek_table.tracks_points[t];
        let payload = if t == 0 { &track0_payload } else { loom_frames_payload };
        
        for i in 0..points.len() {
            let start = points[i].byte_offset as usize;
            let end = if i + 1 < points.len() {
                points[i + 1].byte_offset as usize
            } else {
                payload.len()
            };
            if start < payload.len() && end <= payload.len() {
                let frame_bytes = payload[start..end].to_vec();
                track_frames[t].push(frame_bytes);
            }
        }
    }

    Ok(track_frames)
}

pub fn encode_diff(v1_bytes: &[u8], v2_bytes: &[u8]) -> io::Result<SessionDiff> {
    let mut hasher = Md5::new();
    hasher.update(v1_bytes);
    let mut base_md5 = [0u8; 16];
    base_md5.copy_from_slice(&hasher.finalize());

    let (v2_loom_payload, _, _) = extract_loom_payload_and_track0(v2_bytes)?;
    let mut loom_cursor = Cursor::new(&v2_loom_payload);
    let _header = SessionHeader::deserialize(&mut loom_cursor)?;
    let _seek_table = SeekTable::deserialize(&mut loom_cursor)?;
    let _edit_block = EditBlock::deserialize(&mut loom_cursor)?;
    
    // For GHA/tests, the diff structure expects the metadata payload.
    // Wait, the diff metadata payload in GHA applies to reconstruct the target v2 file.
    // In GHA, apply_diff uses the metadata payload of the diff to rebuild v2.
    // We should extract the actual FLAC metadata blocks (headers) of v2 up to track0 frames!
    let mut cursor = Cursor::new(v2_bytes);
    let mut magic = [0u8; 4];
    cursor.read_exact(&mut magic)?;
    let is_old_format = magic == *b"LSE\x01" || magic == *b"LOOM";
    if !is_old_format {
        loop {
            let mut header = [0u8; 4];
            cursor.read_exact(&mut header)?;
            let is_last = (header[0] & 0x80) != 0;
            let _block_type = header[0] & 0x7F;
            let length = u32::from_be_bytes([0, header[1], header[2], header[3]]) as usize;
            cursor.set_position(cursor.position() + length as u64);
            if is_last {
                break;
            }
        }
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
                instructions.push(FrameInstruction::Insert {
                    frame_bytes: frame_bytes.clone(),
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
