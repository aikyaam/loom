use std::io::{self, Read, Write};

#[derive(Clone, Debug, PartialEq)]
pub struct TrackInfo {
    pub name: String,
    pub total_samples: u64,
    pub md5: [u8; 16],
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionHeader {
    pub sample_rate: u32,
    pub bit_depth: u8,
    pub tracks: Vec<TrackInfo>,
}

impl SessionHeader {
    pub fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(b"LOOM")?;

        writer.write_all(&self.sample_rate.to_be_bytes())?;

        writer.write_all(&[self.bit_depth])?;

        writer.write_all(&(self.tracks.len() as u16).to_be_bytes())?;

        for track in &self.tracks {
            let name_bytes = track.name.as_bytes();
            if name_bytes.len() > 255 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Track name exceeds 255 bytes",
                ));
            }
            writer.write_all(&[name_bytes.len() as u8])?;

            writer.write_all(name_bytes)?;

            writer.write_all(&track.total_samples.to_be_bytes())?;

            writer.write_all(&track.md5)?;
        }

        Ok(())
    }

    pub fn deserialize<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != b"LOOM" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid magic bytes. Not a LOOM stream.",
            ));
        }

        let mut sr_buf = [0u8; 4];
        reader.read_exact(&mut sr_buf)?;
        let sample_rate = u32::from_be_bytes(sr_buf);

        let mut bd_buf = [0u8; 1];
        reader.read_exact(&mut bd_buf)?;
        let bit_depth = bd_buf[0];

        let mut nt_buf = [0u8; 2];
        reader.read_exact(&mut nt_buf)?;
        let num_tracks = u16::from_be_bytes(nt_buf) as usize;

        let mut tracks = Vec::with_capacity(num_tracks);
        for _ in 0..num_tracks {
            let mut name_len_buf = [0u8; 1];
            reader.read_exact(&mut name_len_buf)?;
            let name_len = name_len_buf[0] as usize;

            let mut name_buf = vec![0u8; name_len];
            reader.read_exact(&mut name_buf)?;
            let name = String::from_utf8(name_buf)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            let mut ts_buf = [0u8; 8];
            reader.read_exact(&mut ts_buf)?;
            let total_samples = u64::from_be_bytes(ts_buf);

            let mut md5 = [0u8; 16];
            reader.read_exact(&mut md5)?;

            tracks.push(TrackInfo {
                name,
                total_samples,
                md5,
            });
        }

        Ok(SessionHeader {
            sample_rate,
            bit_depth,
            tracks,
        })
    }
}
