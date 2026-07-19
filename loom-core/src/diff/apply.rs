use crate::diff::container::{FrameInstruction, SessionDiff};
use crate::diff::encode::extract_raw_frames;
use md5::{Digest, Md5};
use std::io::{self, Write};

pub fn apply_diff(v1_bytes: &[u8], diff: &SessionDiff) -> io::Result<Vec<u8>> {
    let mut hasher = Md5::new();
    hasher.update(v1_bytes);
    let computed_md5 = hasher.finalize();
    if computed_md5.as_slice() != diff.base_md5 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Base file MD5 checksum mismatch. Cannot apply diff.",
        ));
    }

    let v1_frames = extract_raw_frames(v1_bytes)?;

    let mut out = Vec::new();

    out.write_all(&diff.metadata_payload)?;

    if diff.tracks_diffs.is_empty() {
        return Ok(out);
    }

    let num_blocks = diff.tracks_diffs[0].instructions.len();
    let num_tracks = diff.tracks_diffs.len();

    for block_idx in 0..num_blocks {
        for t in 0..num_tracks {
            let td = &diff.tracks_diffs[t];
            if block_idx >= td.instructions.len() {
                continue;
            }

            match &td.instructions[block_idx] {
                FrameInstruction::Copy { base_frame_idx } => {
                    let base_idx = *base_frame_idx as usize;
                    if t >= v1_frames.len() || base_idx >= v1_frames[t].len() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Diff copy instruction references out-of-bounds base frame",
                        ));
                    }

                    out.write_all(&v1_frames[t][base_idx])?;
                }
                FrameInstruction::Insert { frame_bytes } => {
                    out.write_all(&(frame_bytes.len() as u32).to_be_bytes())?;
                    out.write_all(frame_bytes)?;
                }
            }
        }
    }

    Ok(out)
}
