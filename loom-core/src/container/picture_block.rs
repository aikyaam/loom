use std::convert::TryInto;
use std::io::{self, Read, Write};

#[derive(Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum PictureType {
    Other = 0,
    FileIcon = 1,
    OtherFileIcon = 2,
    FrontCover = 3,
    BackCover = 4,
    LeafletPage = 5,
    Media = 6,
    LeadArtist = 7,
    ArtistPhoto = 8,
    Conductor = 9,
    Band = 10,
    Composer = 11,
    Lyricist = 12,
    RecordingLocation = 13,
    DuringRecording = 14,
    DuringPerformance = 15,
    MovieScreenCapture = 16,
    ABrightColouredFish = 17,
    Illustration = 18,
    BandLogotype = 19,
    PublisherStudioLogotype = 20,
}

impl PictureType {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => PictureType::FileIcon,
            2 => PictureType::OtherFileIcon,
            3 => PictureType::FrontCover,
            4 => PictureType::BackCover,
            5 => PictureType::LeafletPage,
            6 => PictureType::Media,
            7 => PictureType::LeadArtist,
            8 => PictureType::ArtistPhoto,
            9 => PictureType::Conductor,
            10 => PictureType::Band,
            11 => PictureType::Composer,
            12 => PictureType::Lyricist,
            13 => PictureType::RecordingLocation,
            14 => PictureType::DuringRecording,
            15 => PictureType::DuringPerformance,
            16 => PictureType::MovieScreenCapture,
            17 => PictureType::ABrightColouredFish,
            18 => PictureType::Illustration,
            19 => PictureType::BandLogotype,
            20 => PictureType::PublisherStudioLogotype,
            _ => PictureType::Other,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PictureBlock {
    pub picture_type: PictureType,
    pub mime_type: String,
    pub description: String,
    pub width: u32,
    pub height: u32,
    pub color_depth: u32,
    pub num_colors: u32,
    pub data: Vec<u8>,
}

impl PictureBlock {
    pub fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let mime_bytes = self.mime_type.as_bytes();
        let desc_bytes = self.description.as_bytes();

        let len =
            4 + 4 + mime_bytes.len() + 4 + desc_bytes.len() + 4 + 4 + 4 + 4 + 4 + self.data.len();

        writer.write_all(&(len as u32).to_be_bytes())?;

        let ptype = self.picture_type.clone() as u32;
        writer.write_all(&ptype.to_be_bytes())?;

        writer.write_all(&(mime_bytes.len() as u32).to_be_bytes())?;
        writer.write_all(mime_bytes)?;

        writer.write_all(&(desc_bytes.len() as u32).to_be_bytes())?;
        writer.write_all(desc_bytes)?;

        writer.write_all(&self.width.to_be_bytes())?;
        writer.write_all(&self.height.to_be_bytes())?;
        writer.write_all(&self.color_depth.to_be_bytes())?;
        writer.write_all(&self.num_colors.to_be_bytes())?;

        writer.write_all(&(self.data.len() as u32).to_be_bytes())?;
        writer.write_all(&self.data)?;

        Ok(())
    }

    pub fn deserialize<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut pt_buf = [0u8; 4];
        reader.read_exact(&mut pt_buf)?;
        let picture_type = PictureType::from_u32(u32::from_be_bytes(pt_buf));

        let mut ml_buf = [0u8; 4];
        reader.read_exact(&mut ml_buf)?;
        let mime_len = u32::from_be_bytes(ml_buf) as usize;
        let mut mime_buf = vec![0u8; mime_len];
        reader.read_exact(&mut mime_buf)?;
        let mime_type = String::from_utf8(mime_buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut dl_buf = [0u8; 4];
        reader.read_exact(&mut dl_buf)?;
        let desc_len = u32::from_be_bytes(dl_buf) as usize;
        let mut desc_buf = vec![0u8; desc_len];
        reader.read_exact(&mut desc_buf)?;
        let description = String::from_utf8(desc_buf).unwrap_or_default();

        let mut dims_buf = [0u8; 16];
        reader.read_exact(&mut dims_buf)?;

        let width = u32::from_be_bytes(dims_buf[0..4].try_into().unwrap());
        let height = u32::from_be_bytes(dims_buf[4..8].try_into().unwrap());
        let color_depth = u32::from_be_bytes(dims_buf[8..12].try_into().unwrap());
        let num_colors = u32::from_be_bytes(dims_buf[12..16].try_into().unwrap());

        let mut data_len_buf = [0u8; 4];
        reader.read_exact(&mut data_len_buf)?;
        let data_len = u32::from_be_bytes(data_len_buf) as usize;
        let mut data = vec![0u8; data_len];
        reader.read_exact(&mut data)?;

        Ok(PictureBlock {
            picture_type,
            mime_type,
            description,
            width,
            height,
            color_depth,
            num_colors,
            data,
        })
    }
}
