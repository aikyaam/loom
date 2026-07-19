use crate::edit::schema::{Fade, FadeShape, GainPoint, MuteRegion, TrackEdits};
use std::collections::HashMap;
use std::io::{self, Read, Write};

#[derive(Clone, Debug, PartialEq)]
pub struct EditBlock {
    pub tracks_edits: HashMap<u16, TrackEdits>,
}

impl EditBlock {
    pub fn new() -> Self {
        Self {
            tracks_edits: HashMap::new(),
        }
    }

    pub fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&[0x01])?;

        let mut len = 2;
        for (&_t_idx, edits) in &self.tracks_edits {
            len += 2;
            len += 4;
            len += edits.mutes.len() * (8 + 8);
            len += 4;
            len += edits.fades.len() * (8 + 8 + 1 + 1);
            len += 4;
            len += edits.gain_points.len() * (8 + 4);
        }

        writer.write_all(&(len as u32).to_be_bytes())?;

        writer.write_all(&(self.tracks_edits.len() as u16).to_be_bytes())?;
        for (&t_idx, edits) in &self.tracks_edits {
            writer.write_all(&t_idx.to_be_bytes())?;

            writer.write_all(&(edits.mutes.len() as u32).to_be_bytes())?;
            for mute in &edits.mutes {
                writer.write_all(&mute.start_sample.to_be_bytes())?;
                writer.write_all(&mute.end_sample.to_be_bytes())?;
            }

            writer.write_all(&(edits.fades.len() as u32).to_be_bytes())?;
            for fade in &edits.fades {
                writer.write_all(&fade.start_sample.to_be_bytes())?;
                writer.write_all(&fade.end_sample.to_be_bytes())?;
                writer.write_all(&[fade.shape as u8])?;
                writer.write_all(&[if fade.is_fade_in { 1 } else { 0 }])?;
            }

            writer.write_all(&(edits.gain_points.len() as u32).to_be_bytes())?;
            for pt in &edits.gain_points {
                writer.write_all(&pt.sample_offset.to_be_bytes())?;
                writer.write_all(&pt.gain.to_be_bytes())?;
            }
        }

        Ok(())
    }

    pub fn deserialize<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut nt_buf = [0u8; 2];
        reader.read_exact(&mut nt_buf)?;
        let num_tracks = u16::from_be_bytes(nt_buf) as usize;

        let mut tracks_edits = HashMap::with_capacity(num_tracks);
        for _ in 0..num_tracks {
            let mut ti_buf = [0u8; 2];
            reader.read_exact(&mut ti_buf)?;
            let track_idx = u16::from_be_bytes(ti_buf);
            let mut nm_buf = [0u8; 4];
            reader.read_exact(&mut nm_buf)?;
            let num_mutes = u32::from_be_bytes(nm_buf) as usize;
            let mut mutes = Vec::with_capacity(num_mutes);
            for _ in 0..num_mutes {
                let mut ss_buf = [0u8; 8];
                reader.read_exact(&mut ss_buf)?;
                let start_sample = u64::from_be_bytes(ss_buf);

                let mut es_buf = [0u8; 8];
                reader.read_exact(&mut es_buf)?;
                let end_sample = u64::from_be_bytes(es_buf);

                mutes.push(MuteRegion {
                    start_sample,
                    end_sample,
                });
            }

            let mut nf_buf = [0u8; 4];
            reader.read_exact(&mut nf_buf)?;
            let num_fades = u32::from_be_bytes(nf_buf) as usize;
            let mut fades = Vec::with_capacity(num_fades);
            for _ in 0..num_fades {
                let mut ss_buf = [0u8; 8];
                reader.read_exact(&mut ss_buf)?;
                let start_sample = u64::from_be_bytes(ss_buf);

                let mut es_buf = [0u8; 8];
                reader.read_exact(&mut es_buf)?;
                let end_sample = u64::from_be_bytes(es_buf);

                let mut sh_buf = [0u8; 1];
                reader.read_exact(&mut sh_buf)?;
                let shape = FadeShape::from_code(sh_buf[0]).ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "Invalid fade shape code")
                })?;

                let mut fi_buf = [0u8; 1];
                reader.read_exact(&mut fi_buf)?;
                let is_fade_in = fi_buf[0] != 0;

                fades.push(Fade {
                    start_sample,
                    end_sample,
                    shape,
                    is_fade_in,
                });
            }

            let mut ng_buf = [0u8; 4];
            reader.read_exact(&mut ng_buf)?;
            let num_gpts = u32::from_be_bytes(ng_buf) as usize;
            let mut gain_points = Vec::with_capacity(num_gpts);
            for _ in 0..num_gpts {
                let mut so_buf = [0u8; 8];
                reader.read_exact(&mut so_buf)?;
                let sample_offset = u64::from_be_bytes(so_buf);

                let mut g_buf = [0u8; 4];
                reader.read_exact(&mut g_buf)?;
                let gain = f32::from_be_bytes(g_buf);

                gain_points.push(GainPoint {
                    sample_offset,
                    gain,
                });
            }

            tracks_edits.insert(
                track_idx,
                TrackEdits {
                    mutes,
                    fades,
                    gain_points,
                },
            );
        }

        Ok(EditBlock { tracks_edits })
    }
}
