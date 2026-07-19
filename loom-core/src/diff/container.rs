use std::io::{self, Read, Write};

#[derive(Clone, Debug, PartialEq)]
pub enum FrameInstruction {
    Copy { base_frame_idx: u32 },
    Insert { frame_bytes: Vec<u8> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackDiff {
    pub track_idx: u16,
    pub instructions: Vec<FrameInstruction>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionDiff {
    pub base_md5: [u8; 16],
    pub metadata_payload: Vec<u8>,
    pub tracks_diffs: Vec<TrackDiff>,
}

impl SessionDiff {
    pub fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(b"LDIFF")?;

        writer.write_all(&self.base_md5)?;

        writer.write_all(&(self.metadata_payload.len() as u32).to_be_bytes())?;
        writer.write_all(&self.metadata_payload)?;

        writer.write_all(&(self.tracks_diffs.len() as u16).to_be_bytes())?;

        for td in &self.tracks_diffs {
            writer.write_all(&td.track_idx.to_be_bytes())?;
            writer.write_all(&(td.instructions.len() as u32).to_be_bytes())?;

            for inst in &td.instructions {
                match inst {
                    FrameInstruction::Copy { base_frame_idx } => {
                        writer.write_all(&[0x00])?;
                        writer.write_all(&base_frame_idx.to_be_bytes())?;
                    }
                    FrameInstruction::Insert { frame_bytes } => {
                        writer.write_all(&[0x01])?;
                        writer.write_all(&(frame_bytes.len() as u32).to_be_bytes())?;
                        writer.write_all(frame_bytes)?;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn deserialize<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut magic = [0u8; 5];
        reader.read_exact(&mut magic)?;
        if &magic != b"LDIFF" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid magic bytes for diff",
            ));
        }

        let mut base_md5 = [0u8; 16];
        reader.read_exact(&mut base_md5)?;

        let mut ml_buf = [0u8; 4];
        reader.read_exact(&mut ml_buf)?;
        let metadata_len = u32::from_be_bytes(ml_buf) as usize;
        let mut metadata_payload = vec![0u8; metadata_len];
        reader.read_exact(&mut metadata_payload)?;

        let mut nt_buf = [0u8; 2];
        reader.read_exact(&mut nt_buf)?;
        let num_tracks = u16::from_be_bytes(nt_buf) as usize;

        let mut tracks_diffs = Vec::with_capacity(num_tracks);
        for _ in 0..num_tracks {
            let mut ti_buf = [0u8; 2];
            reader.read_exact(&mut ti_buf)?;
            let track_idx = u16::from_be_bytes(ti_buf);

            let mut ni_buf = [0u8; 4];
            reader.read_exact(&mut ni_buf)?;
            let num_instructions = u32::from_be_bytes(ni_buf) as usize;

            let mut instructions = Vec::with_capacity(num_instructions);
            for _ in 0..num_instructions {
                let mut type_buf = [0u8; 1];
                reader.read_exact(&mut type_buf)?;
                let inst_type = type_buf[0];

                match inst_type {
                    0x00 => {
                        let mut fi_buf = [0u8; 4];
                        reader.read_exact(&mut fi_buf)?;
                        let base_frame_idx = u32::from_be_bytes(fi_buf);
                        instructions.push(FrameInstruction::Copy { base_frame_idx });
                    }
                    0x01 => {
                        let mut fl_buf = [0u8; 4];
                        reader.read_exact(&mut fl_buf)?;
                        let frame_len = u32::from_be_bytes(fl_buf) as usize;
                        let mut frame_bytes = vec![0u8; frame_len];
                        reader.read_exact(&mut frame_bytes)?;
                        instructions.push(FrameInstruction::Insert { frame_bytes });
                    }
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Invalid instruction type",
                        ));
                    }
                }
            }

            tracks_diffs.push(TrackDiff {
                track_idx,
                instructions,
            });
        }

        Ok(SessionDiff {
            base_md5,
            metadata_payload,
            tracks_diffs,
        })
    }
}
