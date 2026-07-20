pub mod analyze;
pub mod bitstream;
pub mod config;
pub mod container;
pub mod crc;
pub mod decoder;
pub mod decorrelate;
pub mod diff;
pub mod edit;
pub mod encoder;
pub mod entropy;
pub mod ffi;
pub mod predict;
pub mod transform;
pub mod verify;

pub use config::{CompressionLevel, EncoderConfig};
pub use container::edit_block::EditBlock;
pub use container::frame::{crc16, Frame};
pub use container::metadata_tags::MetadataTags;
pub use container::padding_block::PaddingBlock;
pub use container::picture_block::{PictureBlock, PictureType};
pub use container::session::{
    decode_session, decode_session_full, encode_session, encode_session_with_config,
};
pub use decoder::{decode_track, decode_track_partial};
pub use diff::{apply_diff, encode_diff, FrameInstruction, SessionDiff, TrackDiff};
pub use edit::{apply_edits, Fade, FadeShape, GainPoint, MuteRegion, TrackEdits};
pub use encoder::{encode_track, encode_track_with_config};
