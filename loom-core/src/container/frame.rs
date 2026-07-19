use crate::bitstream::{BitReader, BitWriter};
use crate::entropy::rice::{decode_residuals, encode_residuals};
use crate::predict::PredictionMode;
use std::io;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChannelLayout {
    Mono,
    Stereo,
    Surround3_0,
    Quad,
    Surround5_0,
    Surround5_1,
    Surround7_0,
    Surround7_1,
    StereoLeftSide,
    StereoRightSide,
    StereoMidSide,
}

impl ChannelLayout {
    pub fn channels(&self) -> u8 {
        match self {
            Self::Mono => 1,
            Self::Stereo | Self::StereoLeftSide | Self::StereoRightSide | Self::StereoMidSide => 2,
            Self::Surround3_0 => 3,
            Self::Quad => 4,
            Self::Surround5_0 => 5,
            Self::Surround5_1 => 6,
            Self::Surround7_0 => 7,
            Self::Surround7_1 => 8,
        }
    }

    pub fn to_code(&self) -> u8 {
        match self {
            Self::Mono => 0,
            Self::Stereo => 1,
            Self::Surround3_0 => 2,
            Self::Quad => 3,
            Self::Surround5_0 => 4,
            Self::Surround5_1 => 5,
            Self::Surround7_0 => 6,
            Self::Surround7_1 => 7,
            Self::StereoLeftSide => 8,
            Self::StereoRightSide => 9,
            Self::StereoMidSide => 10,
        }
    }

    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Mono),
            1 => Some(Self::Stereo),
            2 => Some(Self::Surround3_0),
            3 => Some(Self::Quad),
            4 => Some(Self::Surround5_0),
            5 => Some(Self::Surround5_1),
            6 => Some(Self::Surround7_0),
            7 => Some(Self::Surround7_1),
            8 => Some(Self::StereoLeftSide),
            9 => Some(Self::StereoRightSide),
            10 => Some(Self::StereoMidSide),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Subframe {
    pub mode: PredictionMode,
    pub ref_track: Option<u16>,
    pub ref_weight: i8,
    pub wasted_bits: u8,
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub frame_seq: u32,
    pub block_size: u32,
    pub channel_layout: ChannelLayout,
    pub subframes: Vec<Subframe>,
}

impl Frame {
    pub fn serialize_loom(&self, track_idx: u16, bps: usize, allow_escape: bool) -> Vec<u8> {
        let mut writer = BitWriter::new();

        writer.write_bits(0xF8A5, 16);
        writer.write_bits(track_idx as u64, 16);
        writer.write_bits(self.frame_seq as u32 as u64, 32);
        writer.write_bits(self.block_size as u64, 16);
        writer.write_bits(self.channel_layout.to_code() as u64, 4);

        for (ch, subframe) in self.subframes.iter().enumerate() {
            if let Some(ref_tr) = subframe.ref_track {
                writer.write_bit(true);
                writer.write_bits(ref_tr as u64, 16);
                writer.write_bits((subframe.ref_weight as u8) as u64, 8);
            } else {
                writer.write_bit(false);
            }

            if subframe.wasted_bits > 0 {
                writer.write_bit(true);
                writer.write_bits(subframe.wasted_bits as u64, 5);
            } else {
                writer.write_bit(false);
            }

            let mut subframe_bps = if self.channel_layout.channels() == 2 {
                if (self.channel_layout == ChannelLayout::StereoLeftSide && ch == 1)
                    || (self.channel_layout == ChannelLayout::StereoRightSide && ch == 0)
                    || (self.channel_layout == ChannelLayout::StereoMidSide && ch == 1)
                {
                    bps + 1
                } else {
                    bps
                }
            } else {
                bps
            };

            if subframe_bps > subframe.wasted_bits as usize {
                subframe_bps -= subframe.wasted_bits as usize;
            } else {
                subframe_bps = 1;
            }

            match &subframe.mode {
                PredictionMode::Constant(val) => {
                    writer.write_bits(0, 3);
                    let mask = (1u64 << subframe_bps) - 1;
                    writer.write_bits((*val as u64) & mask, subframe_bps);
                }
                PredictionMode::Verbatim(samples) => {
                    writer.write_bits(1, 3);
                    let mask = (1u64 << subframe_bps) - 1;
                    for &val in samples {
                        writer.write_bits((val as u64) & mask, subframe_bps);
                    }
                }
                PredictionMode::Fixed { order, residuals } => {
                    writer.write_bits(2, 3);
                    writer.write_bits(*order as u64, 3);

                    let mask = (1u64 << subframe_bps) - 1;
                    for i in 0..*order {
                        writer.write_bits((residuals[i] as u64) & mask, subframe_bps);
                    }
                    encode_residuals(&mut writer, residuals, *order, 2, allow_escape);
                }
                PredictionMode::Lpc {
                    order,
                    qlp_coeffs,
                    qlp_shift,
                    qlp_precision,
                    residuals,
                } => {
                    writer.write_bits(3, 3);
                    writer.write_bits(*order as u64, 6);
                    writer.write_bits(*qlp_precision as u64, 8);
                    writer.write_bits((*qlp_shift as u8) as u64, 8);

                    let mask_prec = (1u64 << qlp_precision) - 1;
                    for &coeff in qlp_coeffs {
                        writer.write_bits((coeff as u64) & mask_prec, *qlp_precision);
                    }

                    let mask_bps = (1u64 << subframe_bps) - 1;
                    for i in 0..*order {
                        writer.write_bits((residuals[i] as u64) & mask_bps, subframe_bps);
                    }

                    encode_residuals(&mut writer, residuals, *order, 2, allow_escape);
                }
            }
        }

        let mut bytes = writer.into_bytes();
        let crc = crate::crc::flac_crc16(&bytes);
        bytes.push((crc >> 8) as u8);
        bytes.push((crc & 0xFF) as u8);
        bytes
    }

    pub fn serialize_flac(&self, sample_number: u64, bps: usize, allow_escape: bool) -> Vec<u8> {
        let mut writer = BitWriter::new();

        writer.write_bits(0x3FFE, 14);
        writer.write_bit(false);
        writer.write_bit(true);
        writer.write_bits(7, 4);
        writer.write_bits(0, 4);
        writer.write_bits(self.channel_layout.to_code() as u64, 4);
        writer.write_bits(0, 3);
        writer.write_bit(false);

        writer.write_utf8_uint(sample_number);
        writer.write_bits((self.block_size - 1) as u64, 16);

        writer.flush();
        let header_bytes = writer.bytes.clone();
        let crc8 = crate::crc::flac_crc8(&header_bytes);
        writer.write_bits(crc8 as u64, 8);

        for (ch, subframe) in self.subframes.iter().enumerate() {
            writer.write_bit(false);

            let mode_code = match &subframe.mode {
                PredictionMode::Constant(_) => 0x00,
                PredictionMode::Verbatim(_) => 0x01,
                PredictionMode::Fixed { order, .. } => 0x08 | (*order as u64 & 0x07),
                PredictionMode::Lpc { order, .. } => 0x20 | ((*order as u64 - 1) & 0x1F),
            };
            writer.write_bits(mode_code, 6);

            if subframe.wasted_bits > 0 {
                writer.write_bit(true);
                writer.write_unary((subframe.wasted_bits - 1) as u64);
            } else {
                writer.write_bit(false);
            }

            let mut subframe_bps = if self.channel_layout.channels() == 2 {
                if (self.channel_layout == ChannelLayout::StereoLeftSide && ch == 1)
                    || (self.channel_layout == ChannelLayout::StereoRightSide && ch == 0)
                    || (self.channel_layout == ChannelLayout::StereoMidSide && ch == 1)
                {
                    bps + 1
                } else {
                    bps
                }
            } else {
                bps
            };

            if subframe_bps > subframe.wasted_bits as usize {
                subframe_bps -= subframe.wasted_bits as usize;
            } else {
                subframe_bps = 1;
            }

            match &subframe.mode {
                PredictionMode::Constant(val) => {
                    let mask = (1u64 << subframe_bps) - 1;
                    writer.write_bits((*val as u64) & mask, subframe_bps);
                }
                PredictionMode::Verbatim(samples) => {
                    let mask = (1u64 << subframe_bps) - 1;
                    for &val in samples {
                        writer.write_bits((val as u64) & mask, subframe_bps);
                    }
                }
                PredictionMode::Fixed { order, residuals } => {
                    let mask = (1u64 << subframe_bps) - 1;
                    for i in 0..*order {
                        writer.write_bits((residuals[i] as u64) & mask, subframe_bps);
                    }
                    encode_residuals(&mut writer, residuals, *order, 2, allow_escape);
                }
                PredictionMode::Lpc {
                    order,
                    qlp_coeffs,
                    qlp_shift,
                    qlp_precision,
                    residuals,
                } => {
                    let mask_bps = (1u64 << subframe_bps) - 1;
                    for i in 0..*order {
                        writer.write_bits((residuals[i] as u64) & mask_bps, subframe_bps);
                    }

                    writer.write_bits(*qlp_precision as u64 - 1, 4);

                    let shift = *qlp_shift as i8;
                    writer.write_bits((shift as u64) & 0x1F, 5);

                    let mask_prec = (1u64 << qlp_precision) - 1;
                    for &coeff in qlp_coeffs {
                        writer.write_bits((coeff as u64) & mask_prec, *qlp_precision);
                    }

                    encode_residuals(&mut writer, residuals, *order, 2, allow_escape);
                }
            }
        }

        writer.align_to_byte();
        writer.flush();
        let mut bytes = writer.bytes.clone();
        let crc16 = crate::crc::flac_crc16(&bytes);
        bytes.push((crc16 >> 8) as u8);
        bytes.push((crc16 & 0xFF) as u8);
        bytes
    }

    pub fn deserialize_loom(
        reader: &mut BitReader,
        bps: usize,
        _decoded_pcm: &mut [Vec<i64>],
    ) -> io::Result<(Self, u16)> {
        if reader.bits_left() < 100 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Frame header truncated",
            ));
        }

        let all_bytes = reader.peek_remaining_bytes();
        if all_bytes.len() >= 2 {
            let payload = &all_bytes[..all_bytes.len() - 2];
            let stored_crc = ((all_bytes[all_bytes.len() - 2] as u16) << 8)
                | (all_bytes[all_bytes.len() - 1] as u16);
            let computed_crc = crc16(payload);
            if stored_crc != computed_crc {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "CRC-16 mismatch: stored={:#06x}, computed={:#06x}",
                        stored_crc, computed_crc
                    ),
                ));
            }
        }

        let sync_word = reader.read_bits(16)?;
        if sync_word != 0xF8A5 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Sync marker mismatch",
            ));
        }

        let track_idx = reader.read_bits(16)? as u16;
        let frame_seq = reader.read_bits(32)? as u32;
        let block_size = reader.read_bits(16)? as u32;
        let layout_code = reader.read_bits(4)? as u8;

        let channel_layout = ChannelLayout::from_code(layout_code).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "Invalid channel layout code")
        })?;
        let channels = channel_layout.channels();

        let mut subframes = Vec::with_capacity(channels as usize);

        for ch in 0..(channels as usize) {
            let has_cross_ref = reader.read_bit()?;
            let (ref_track, ref_weight) = if has_cross_ref {
                let r_track = reader.read_bits(16)? as u16;
                let r_weight = reader.read_bits(8)? as u8 as i8;
                (Some(r_track), r_weight)
            } else {
                (None, 0)
            };

            let has_wasted_bits = reader.read_bit()?;
            let wasted_bits = if has_wasted_bits {
                reader.read_bits(5)? as u8
            } else {
                0
            };

            let mut subframe_bps = if channels == 2
                && (channel_layout == ChannelLayout::StereoLeftSide
                    || channel_layout == ChannelLayout::StereoRightSide
                    || channel_layout == ChannelLayout::StereoMidSide)
                && ch == 1
            {
                bps + 1
            } else {
                bps
            };

            if subframe_bps > wasted_bits as usize {
                subframe_bps -= wasted_bits as usize;
            } else {
                subframe_bps = 1;
            }

            let mode_code = reader.read_bits(3)?;
            let mode = match mode_code {
                0 => {
                    let sign_bit = 1u64 << (subframe_bps - 1);
                    let mask = (1u64 << subframe_bps) - 1;
                    let uval = reader.read_bits(subframe_bps)?;
                    let sval = if (uval & sign_bit) != 0 {
                        (uval | !mask) as i64
                    } else {
                        uval as i64
                    };
                    PredictionMode::Constant(sval)
                }
                1 => {
                    let mut samples = vec![0i64; block_size as usize];
                    let sign_bit = 1u64 << (subframe_bps - 1);
                    let mask = (1u64 << subframe_bps) - 1;
                    for i in 0..(block_size as usize) {
                        let uval = reader.read_bits(subframe_bps)?;
                        let sval = if (uval & sign_bit) != 0 {
                            (uval | !mask) as i64
                        } else {
                            uval as i64
                        };
                        samples[i] = sval;
                    }
                    PredictionMode::Verbatim(samples)
                }
                2 => {
                    let order = reader.read_bits(3)? as usize;
                    let mut residuals = vec![0i64; block_size as usize];

                    let sign_bit = 1u64 << (subframe_bps - 1);
                    let mask = (1u64 << subframe_bps) - 1;
                    for i in 0..order {
                        let uval = reader.read_bits(subframe_bps)?;
                        let sval = if (uval & sign_bit) != 0 {
                            (uval | !mask) as i64
                        } else {
                            uval as i64
                        };
                        residuals[i] = sval;
                    }

                    decode_residuals(reader, &mut residuals, order)?;
                    PredictionMode::Fixed { order, residuals }
                }
                3 => {
                    let order = reader.read_bits(6)? as usize;
                    let qlp_precision = reader.read_bits(8)? as usize;
                    let qlp_shift = reader.read_bits(8)? as u8 as i8;

                    let mut qlp_coeffs = Vec::with_capacity(order);
                    let coeff_sign_bit = 1u64 << (qlp_precision - 1);
                    let coeff_mask = (1u64 << qlp_precision) - 1;
                    for _ in 0..order {
                        let uval = reader.read_bits(qlp_precision)?;
                        let sval = if (uval & coeff_sign_bit) != 0 {
                            (uval | !coeff_mask) as i32
                        } else {
                            uval as i32
                        };
                        qlp_coeffs.push(sval);
                    }

                    let mut residuals = vec![0i64; block_size as usize];
                    let sign_bit = 1u64 << (subframe_bps - 1);
                    let mask = (1u64 << subframe_bps) - 1;
                    for i in 0..order {
                        let uval = reader.read_bits(subframe_bps)?;
                        let sval = if (uval & sign_bit) != 0 {
                            (uval | !mask) as i64
                        } else {
                            uval as i64
                        };
                        residuals[i] = sval;
                    }

                    decode_residuals(reader, &mut residuals, order)?;
                    PredictionMode::Lpc {
                        order,
                        qlp_coeffs,
                        qlp_shift,
                        qlp_precision,
                        residuals,
                    }
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid subframe mode code: {}", mode_code),
                    ));
                }
            };

            subframes.push(Subframe {
                mode,
                ref_track,
                ref_weight,
                wasted_bits,
            });
        }

        reader.align_to_byte();
        Ok((
            Frame {
                frame_seq,
                block_size,
                channel_layout,
                subframes,
            },
            track_idx,
        ))
    }

    pub fn deserialize_flac(
        reader: &mut BitReader,
        bps: usize,
        _decoded_pcm: &mut [Vec<i64>],
    ) -> io::Result<Self> {
        let sync_word = reader.read_bits(14)?;
        if sync_word != 0x3FFE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "FLAC Sync marker mismatch",
            ));
        }

        reader.read_bit()?;
        let _blocking_strategy = reader.read_bit()?;
        let block_size_code = reader.read_bits(4)?;
        let _sample_rate_code = reader.read_bits(4)?;
        let layout_code = reader.read_bits(4)? as u8;
        let _sample_size_code = reader.read_bits(3)?;
        reader.read_bit()?;

        let frame_seq = reader.read_utf8_uint()? as u32;

        let block_size = if block_size_code == 7 {
            (reader.read_bits(16)? + 1) as u32
        } else if block_size_code == 6 {
            (reader.read_bits(8)? + 1) as u32
        } else if block_size_code == 1 {
            192
        } else {
            0
        };

        let _crc8 = reader.read_bits(8)?;

        let channel_layout = ChannelLayout::from_code(layout_code).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "Invalid channel layout code")
        })?;
        let channels = channel_layout.channels();

        let mut subframes = Vec::with_capacity(channels as usize);

        for ch in 0..(channels as usize) {
            reader.read_bit()?;

            let mode_code = reader.read_bits(6)?;

            let has_wasted_bits = reader.read_bit()?;
            let wasted_bits = if has_wasted_bits {
                (reader.read_unary()? + 1) as u8
            } else {
                0
            };

            let mut subframe_bps = if channels == 2
                && (channel_layout == ChannelLayout::StereoLeftSide
                    || channel_layout == ChannelLayout::StereoRightSide
                    || channel_layout == ChannelLayout::StereoMidSide)
                && ch == 1
            {
                bps + 1
            } else {
                bps
            };

            if subframe_bps > wasted_bits as usize {
                subframe_bps -= wasted_bits as usize;
            } else {
                subframe_bps = 1;
            }

            let mode = if mode_code == 0 {
                let sign_bit = 1u64 << (subframe_bps - 1);
                let mask = (1u64 << subframe_bps) - 1;
                let uval = reader.read_bits(subframe_bps)?;
                let sval = if (uval & sign_bit) != 0 {
                    (uval | !mask) as i64
                } else {
                    uval as i64
                };
                PredictionMode::Constant(sval)
            } else if mode_code == 1 {
                let mut samples = vec![0i64; block_size as usize];
                let sign_bit = 1u64 << (subframe_bps - 1);
                let mask = (1u64 << subframe_bps) - 1;
                for i in 0..(block_size as usize) {
                    let uval = reader.read_bits(subframe_bps)?;
                    let sval = if (uval & sign_bit) != 0 {
                        (uval | !mask) as i64
                    } else {
                        uval as i64
                    };
                    samples[i] = sval;
                }
                PredictionMode::Verbatim(samples)
            } else if mode_code >= 0x08 && mode_code <= 0x0F {
                let order = (mode_code & 0x07) as usize;
                let mut residuals = vec![0i64; block_size as usize];
                let sign_bit = 1u64 << (subframe_bps - 1);
                let mask = (1u64 << subframe_bps) - 1;
                for i in 0..order {
                    let uval = reader.read_bits(subframe_bps)?;
                    let sval = if (uval & sign_bit) != 0 {
                        (uval | !mask) as i64
                    } else {
                        uval as i64
                    };
                    residuals[i] = sval;
                }
                decode_residuals(reader, &mut residuals, order)?;
                PredictionMode::Fixed { order, residuals }
            } else if mode_code >= 0x20 {
                let order = ((mode_code & 0x1F) + 1) as usize;

                let mut residuals = vec![0i64; block_size as usize];
                let bps_sign_bit = 1u64 << (subframe_bps - 1);
                let bps_mask = (1u64 << subframe_bps) - 1;
                for i in 0..order {
                    let uval = reader.read_bits(subframe_bps)?;
                    let sval = if (uval & bps_sign_bit) != 0 {
                        (uval | !bps_mask) as i64
                    } else {
                        uval as i64
                    };
                    residuals[i] = sval;
                }

                let qlp_precision = (reader.read_bits(4)? + 1) as usize;

                let shift_u = reader.read_bits(5)?;
                let qlp_shift = if (shift_u & 0x10) != 0 {
                    (shift_u | !0x1F) as i8
                } else {
                    shift_u as i8
                };

                let mut qlp_coeffs = vec![0i32; order];
                let sign_bit = 1u64 << (qlp_precision - 1);
                let mask = (1u64 << qlp_precision) - 1;
                for i in 0..order {
                    let uval = reader.read_bits(qlp_precision)?;
                    let sval = if (uval & sign_bit) != 0 {
                        (uval | !mask) as i32
                    } else {
                        uval as i32
                    };
                    qlp_coeffs[i] = sval;
                }

                decode_residuals(reader, &mut residuals, order)?;
                PredictionMode::Lpc {
                    order,
                    qlp_coeffs,
                    qlp_shift,
                    qlp_precision,
                    residuals,
                }
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid subframe mode",
                ));
            };

            subframes.push(Subframe {
                mode,
                ref_track: None,
                ref_weight: 0,
                wasted_bits,
            });
        }

        reader.align_to_byte();
        reader.read_bits(16)?;

        Ok(Frame {
            frame_seq,
            block_size,
            channel_layout,
            subframes,
        })
    }

    pub fn scan_for_sync(data: &[u8], start_offset: usize) -> Option<usize> {
        let n = data.len();
        if start_offset + 2 > n {
            return None;
        }
        for i in start_offset..=(n - 2) {
            if data[i] == 0xF8 && data[i + 1] == 0xA5 {
                return Some(i);
            }
        }
        None
    }
}

trait CustomOptionExt<T> {
    fn ok_ok(self, err: &str) -> io::Result<T>;
}

impl<T> CustomOptionExt<T> for Option<T> {
    fn ok_ok(self, err: &str) -> io::Result<T> {
        self.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
    }
}

pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x8005;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}
