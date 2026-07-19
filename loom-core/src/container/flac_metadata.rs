use std::io::{self, Write};

pub struct StreamInfo {
    pub min_block_size: u16,
    pub max_block_size: u16,
    pub min_frame_size: u32,
    pub max_frame_size: u32,
    pub sample_rate: u32,
    pub channels: u8,
    pub bit_depth: u8,
    pub total_samples: u64,
    pub md5: [u8; 16],
}

impl StreamInfo {
    pub fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.min_block_size.to_be_bytes())?;
        writer.write_all(&self.max_block_size.to_be_bytes())?;

        let min_fs = self.min_frame_size.to_be_bytes();
        writer.write_all(&min_fs[1..4])?;

        let max_fs = self.max_frame_size.to_be_bytes();
        writer.write_all(&max_fs[1..4])?;

        let mut packed = 0u64;
        packed |= (self.sample_rate as u64 & 0xFFFFF) << 44;
        packed |= ((self.channels as u64 - 1) & 0x7) << 41;
        packed |= ((self.bit_depth as u64 - 1) & 0x1F) << 36;
        packed |= self.total_samples & 0xFFFFFFFFF;

        writer.write_all(&packed.to_be_bytes())?;
        writer.write_all(&self.md5)?;

        Ok(())
    }
}

pub fn write_metadata_block_header<W: Write>(
    writer: &mut W,
    is_last: bool,
    block_type: u8,
    length: u32,
) -> io::Result<()> {
    let mut header = [0u8; 4];
    header[0] = (block_type & 0x7F) | if is_last { 0x80 } else { 0x00 };
    let len_bytes = length.to_be_bytes();
    header[1..4].copy_from_slice(&len_bytes[1..4]);
    writer.write_all(&header)?;
    Ok(())
}
