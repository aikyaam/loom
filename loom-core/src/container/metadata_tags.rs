use std::collections::HashMap;
use std::io::{self, Read, Write};

#[derive(Clone, Debug, PartialEq)]
pub struct MetadataTags {
    pub tags: HashMap<String, String>,
}

impl Default for MetadataTags {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataTags {
    pub fn new() -> Self {
        Self {
            tags: HashMap::new(),
        }
    }

    pub fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let vendor = "Loom Audio Codec";
        let vendor_bytes = vendor.as_bytes();

        let mut comments = Vec::new();
        for (k, v) in &self.tags {
            let comment = format!("{}={}", k.to_uppercase(), v);
            comments.push(comment.into_bytes());
        }

        let mut len = 4 + vendor_bytes.len() + 4;
        for c in &comments {
            len += 4 + c.len();
        }

        writer.write_all(&(len as u32).to_be_bytes())?;

        writer.write_all(&(vendor_bytes.len() as u32).to_le_bytes())?;
        writer.write_all(vendor_bytes)?;

        writer.write_all(&(comments.len() as u32).to_le_bytes())?;

        for c in &comments {
            writer.write_all(&(c.len() as u32).to_le_bytes())?;
            writer.write_all(c)?;
        }

        Ok(())
    }

    pub fn deserialize<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut tags = HashMap::new();

        let mut vl_buf = [0u8; 4];
        reader.read_exact(&mut vl_buf)?;
        let vendor_len = u32::from_le_bytes(vl_buf) as usize;
        let mut vendor_buf = vec![0u8; vendor_len];
        reader.read_exact(&mut vendor_buf)?;

        let mut cl_buf = [0u8; 4];
        reader.read_exact(&mut cl_buf)?;
        let num_comments = u32::from_le_bytes(cl_buf) as usize;

        for _ in 0..num_comments {
            let mut l_buf = [0u8; 4];
            reader.read_exact(&mut l_buf)?;
            let c_len = u32::from_le_bytes(l_buf) as usize;

            let mut c_buf = vec![0u8; c_len];
            reader.read_exact(&mut c_buf)?;

            let comment = String::from_utf8(c_buf)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            if let Some((k, v)) = comment.split_once('=') {
                tags.insert(k.to_string(), v.to_string());
            } else {
            }
        }

        Ok(MetadataTags { tags })
    }
}
