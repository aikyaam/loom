use std::io::{self, Read, Write};

#[derive(Clone, Debug, PartialEq)]
pub struct SeekPoint {
    pub sample_number: u64,
    pub byte_offset: u64,
    pub frame_samples: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeekTable {
    pub tracks_points: Vec<Vec<SeekPoint>>,
}

impl SeekTable {
    pub fn new(num_tracks: usize) -> Self {
        Self {
            tracks_points: vec![Vec::new(); num_tracks],
        }
    }

    pub fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&[0x00])?;

        let mut len = 2;
        for points in &self.tracks_points {
            len += 4;
            len += points.len() * (8 + 8 + 4);
        }

        writer.write_all(&(len as u32).to_be_bytes())?;

        writer.write_all(&(self.tracks_points.len() as u16).to_be_bytes())?;
        for points in &self.tracks_points {
            writer.write_all(&(points.len() as u32).to_be_bytes())?;
            for pt in points {
                writer.write_all(&pt.sample_number.to_be_bytes())?;
                writer.write_all(&pt.byte_offset.to_be_bytes())?;
                writer.write_all(&pt.frame_samples.to_be_bytes())?;
            }
        }

        Ok(())
    }

    pub fn deserialize<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut magic = [0u8; 1];
        reader.read_exact(&mut magic)?;
        if magic[0] != 0x00 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid SeekTable magic identifier",
            ));
        }

        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf)?;

        let mut nt_buf = [0u8; 2];
        reader.read_exact(&mut nt_buf)?;
        let num_tracks = u16::from_be_bytes(nt_buf) as usize;

        let mut tracks_points = Vec::with_capacity(num_tracks);
        for _ in 0..num_tracks {
            let mut np_buf = [0u8; 4];
            reader.read_exact(&mut np_buf)?;
            let num_points = u32::from_be_bytes(np_buf) as usize;

            let mut points = Vec::with_capacity(num_points);
            for _ in 0..num_points {
                let mut sn_buf = [0u8; 8];
                reader.read_exact(&mut sn_buf)?;
                let sample_number = u64::from_be_bytes(sn_buf);

                let mut bo_buf = [0u8; 8];
                reader.read_exact(&mut bo_buf)?;
                let byte_offset = u64::from_be_bytes(bo_buf);

                let mut fs_buf = [0u8; 4];
                reader.read_exact(&mut fs_buf)?;
                let frame_samples = u32::from_be_bytes(fs_buf);

                points.push(SeekPoint {
                    sample_number,
                    byte_offset,
                    frame_samples,
                });
            }
            tracks_points.push(points);
        }

        Ok(SeekTable { tracks_points })
    }
}
