use std::io::{self, Read, Write};

#[derive(Clone, Debug, PartialEq)]
pub struct PaddingBlock {
    pub length: u32,
}

impl PaddingBlock {
    pub fn new(length: u32) -> Self {
        Self { length }
    }

    pub fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&[0x04])?;
        writer.write_all(&self.length.to_be_bytes())?;

        let chunk_size = 4096;
        let mut remaining = self.length as usize;
        let zero_chunk = vec![0u8; std::cmp::min(chunk_size, remaining)];
        while remaining > 0 {
            let to_write = std::cmp::min(remaining, zero_chunk.len());
            writer.write_all(&zero_chunk[..to_write])?;
            remaining -= to_write;
        }
        Ok(())
    }

    pub fn deserialize<R: Read>(reader: &mut R, length: u32) -> io::Result<Self> {
        let chunk_size = 4096;
        let mut remaining = length as usize;
        let mut buf = vec![0u8; std::cmp::min(chunk_size, remaining)];
        while remaining > 0 {
            let to_read = std::cmp::min(remaining, buf.len());
            reader.read_exact(&mut buf[..to_read])?;
            remaining -= to_read;
        }
        Ok(Self { length })
    }
}
