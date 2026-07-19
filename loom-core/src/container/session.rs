use crate::bitstream::BitReader;
use crate::container::edit_block::EditBlock;
use crate::container::frame::{Frame, Subframe};
use crate::container::header::{SessionHeader, TrackInfo};
use crate::container::metadata_tags::MetadataTags;
use crate::container::picture_block::PictureBlock;
use crate::container::seek_index::{SeekPoint, SeekTable};
use crate::decorrelate::cross_track::{
    apply_cross_prediction, calculate_cross_coupling, reconstruct_cross_prediction,
};
use crate::decorrelate::stereo::{reconstruct_stereo, search_stereo_mode, StereoMode};
use crate::edit::apply_edits;
use crate::predict::{search_predictor, PredictionMode};
use crate::verify::{compute_pcm_md5, compute_wasted_bits};
use std::io::{self, Cursor, Read, Write};

pub fn encode_session(
    tracks: &[Vec<Vec<i64>>],
    track_names: &[String],
    sample_rate: u32,
    bit_depth: u8,
    block_size: usize,
    edits: Option<&crate::container::edit_block::EditBlock>,
    tags: Option<&MetadataTags>,
    picture: Option<&PictureBlock>,
) -> io::Result<Vec<u8>> {
    if tracks.is_empty() || tracks[0].is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "No audio tracks/channels",
        ));
    }

    let num_tracks = tracks.len();
    let total_samples = tracks[0][0].len();
    let bps = bit_depth as usize;

    let mut track_infos = Vec::with_capacity(num_tracks);
    for t in 0..num_tracks {
        let ch_count = tracks[t].len();
        let mut interleaved = Vec::with_capacity(total_samples * ch_count);
        for s in 0..total_samples {
            for ch in 0..ch_count {
                interleaved.push(tracks[t][ch][s]);
            }
        }
        let md5 = compute_pcm_md5(&interleaved, bit_depth);
        track_infos.push(TrackInfo {
            name: track_names[t].clone(),
            total_samples: total_samples as u64,
            md5,
        });
    }

    let header = SessionHeader {
        sample_rate,
        bit_depth,
        tracks: track_infos,
    };

    let mut track0_frames_buf = Vec::new();
    let mut loom_frames_buf = Vec::new();
    let mut seek_table = SeekTable::new(num_tracks);
    let mut current_samples = vec![0u64; num_tracks];

    let mut offset = 0;
    let mut frame_seq = 0u32;

    let mut min_block_size = u16::MAX;
    let mut max_block_size = 0u16;
    let mut min_frame_size = u32::MAX;
    let mut max_frame_size = 0u32;

    while offset < total_samples {
        let mut current_block_size = std::cmp::min(block_size as usize, total_samples - offset);

        let mut earliest_transient = None;
        for t in 0..num_tracks {
            let track_ch_count = tracks[t].len();
            for ch in 0..track_ch_count {
                if let Some(transient_idx) = crate::analyze::detect_transient(
                    &tracks[t][ch][offset..(offset + current_block_size)],
                ) {
                    let split_at = std::cmp::max(128, transient_idx.saturating_sub(64));
                    if let Some(earliest) = earliest_transient {
                        earliest_transient = Some(std::cmp::min(earliest, split_at));
                    } else {
                        earliest_transient = Some(split_at);
                    }
                }
            }
        }

        if let Some(split) = earliest_transient {
            if split < current_block_size {
                current_block_size = split;
            }
        }

        if (current_block_size as u16) < min_block_size {
            min_block_size = current_block_size as u16;
        }
        if (current_block_size as u16) > max_block_size {
            max_block_size = current_block_size as u16;
        }

        let mut ref_residuals: Option<Vec<i64>> = None;

        for t in 0..num_tracks {
            let track_ch_count = tracks[t].len();
            let mut block_channels = vec![vec![0i64; current_block_size]; track_ch_count];
            for ch in 0..track_ch_count {
                block_channels[ch]
                    .copy_from_slice(&tracks[t][ch][offset..(offset + current_block_size)]);
            }

            let mut subframes = Vec::new();
            let mut stereo_mode = StereoMode::Independent;
            if track_ch_count == 2 {
                let (sm, ch0, ch1) = search_stereo_mode(&block_channels[0], &block_channels[1]);
                stereo_mode = sm;

                let w0 = compute_wasted_bits(&ch0);
                let shifted0 = if w0 > 0 {
                    ch0.iter().map(|&x| x >> w0).collect()
                } else {
                    ch0.clone()
                };
                let mut mode0 = search_predictor(&shifted0, bps - w0 as usize);

                let mut ref_track = None;
                let mut ref_weight = 0i8;

                if t > 0 {
                    if let Some(ref_res) = &ref_residuals {
                        if let PredictionMode::Fixed { residuals, .. } = &mut mode0 {
                            let (w, saved) = calculate_cross_coupling(residuals, ref_res);
                            if saved > 8 {
                                apply_cross_prediction(residuals, ref_res, w);
                                ref_track = Some(0);
                                ref_weight = w;
                            }
                        } else if let PredictionMode::Lpc { residuals, .. } = &mut mode0 {
                            let (w, saved) = calculate_cross_coupling(residuals, ref_res);
                            if saved > 8 {
                                apply_cross_prediction(residuals, ref_res, w);
                                ref_track = Some(0);
                                ref_weight = w;
                            }
                        }
                    }
                }

                subframes.push(Subframe {
                    mode: mode0,
                    ref_track,
                    ref_weight,
                    wasted_bits: w0,
                });

                let w1 = compute_wasted_bits(&ch1);
                let side_bps = if stereo_mode == StereoMode::Independent {
                    bps
                } else {
                    bps + 1
                };
                let shifted1 = if w1 > 0 {
                    ch1.iter().map(|&x| x >> w1).collect()
                } else {
                    ch1.clone()
                };
                let mode1 = search_predictor(&shifted1, side_bps - w1 as usize);
                subframes.push(Subframe {
                    mode: mode1,
                    ref_track: None,
                    ref_weight: 0,
                    wasted_bits: w1,
                });
            } else {
                for ch in 0..track_ch_count {
                    let ch_data = &block_channels[ch];
                    let w = compute_wasted_bits(ch_data);
                    let shifted = if w > 0 {
                        ch_data.iter().map(|&x| x >> w).collect()
                    } else {
                        ch_data.clone()
                    };
                    let mut mode = search_predictor(&shifted, bps - w as usize);

                    let mut ref_track = None;
                    let mut ref_weight = 0i8;

                    if t > 0 && ch == 0 {
                        if let Some(ref_res) = &ref_residuals {
                            if let PredictionMode::Fixed { residuals, .. } = &mut mode {
                                let (w, saved) = calculate_cross_coupling(residuals, ref_res);
                                if saved > 8 {
                                    apply_cross_prediction(residuals, ref_res, w);
                                    ref_track = Some(0);
                                    ref_weight = w;
                                }
                            } else if let PredictionMode::Lpc { residuals, .. } = &mut mode {
                                let (w, saved) = calculate_cross_coupling(residuals, ref_res);
                                if saved > 8 {
                                    apply_cross_prediction(residuals, ref_res, w);
                                    ref_track = Some(0);
                                    ref_weight = w;
                                }
                            }
                        }
                    }

                    if t == 0 && ch == 0 {
                        if let PredictionMode::Fixed { residuals, .. } = &mode {
                            ref_residuals = Some(residuals.clone());
                        } else if let PredictionMode::Lpc { residuals, .. } = &mode {
                            ref_residuals = Some(residuals.clone());
                        }
                    }

                    subframes.push(Subframe {
                        mode,
                        ref_track,
                        ref_weight,
                        wasted_bits: w,
                    });
                }
            }

            use crate::container::frame::ChannelLayout;
            let channel_layout = match track_ch_count {
                1 => ChannelLayout::Mono,
                2 => match stereo_mode {
                    StereoMode::Independent => ChannelLayout::Stereo,
                    StereoMode::LeftSide => ChannelLayout::StereoLeftSide,
                    StereoMode::RightSide => ChannelLayout::StereoRightSide,
                    StereoMode::MidSide => ChannelLayout::StereoMidSide,
                },
                3 => ChannelLayout::Surround3_0,
                4 => ChannelLayout::Quad,
                5 => ChannelLayout::Surround5_0,
                6 => ChannelLayout::Surround5_1,
                7 => ChannelLayout::Surround7_0,
                8 => ChannelLayout::Surround7_1,
                _ => ChannelLayout::Mono,
            };

            let frame = Frame {
                frame_seq,
                block_size: current_block_size as u32,
                channel_layout,
                subframes,
            };

            if t == 0 {
                let frame_offset = track0_frames_buf.len() as u64;
                seek_table.tracks_points[0].push(SeekPoint {
                    sample_number: current_samples[0],
                    byte_offset: frame_offset,
                    frame_samples: current_block_size as u32,
                });

                let frame_bytes = frame.serialize_flac(current_samples[0], bps);
                let fsize = frame_bytes.len() as u32;
                if fsize < min_frame_size {
                    min_frame_size = fsize;
                }
                if fsize > max_frame_size {
                    max_frame_size = fsize;
                }
                track0_frames_buf.extend_from_slice(&frame_bytes);
            } else {
                let frame_offset = loom_frames_buf.len() as u64;
                seek_table.tracks_points[t].push(SeekPoint {
                    sample_number: current_samples[t],
                    byte_offset: frame_offset,
                    frame_samples: current_block_size as u32,
                });

                let frame_bytes = frame.serialize_loom(t as u16, bps);
                loom_frames_buf.extend_from_slice(&(frame_bytes.len() as u32).to_be_bytes());
                loom_frames_buf.extend_from_slice(&frame_bytes);
            }

            current_samples[t] += current_block_size as u64;
        }

        frame_seq += 1;
        offset += current_block_size;
    }

    use crate::container::flac_metadata::{write_metadata_block_header, StreamInfo};
    let mut out = Vec::new();
    out.write_all(b"fLaC")?;

    let streaminfo = StreamInfo {
        min_block_size,
        max_block_size,
        min_frame_size,
        max_frame_size,
        sample_rate,
        channels: tracks[0].len() as u8,
        bit_depth,
        total_samples: header.tracks[0].total_samples,
        md5: header.tracks[0].md5,
    };

    write_metadata_block_header(&mut out, false, 0, 34)?;
    streaminfo.serialize(&mut out)?;

    if let Some(t) = tags {
        let mut t_buf = Vec::new();
        t.serialize(&mut t_buf)?;
        write_metadata_block_header(&mut out, false, 4, t_buf.len() as u32)?;
        out.write_all(&t_buf)?;
    }

    if let Some(p) = picture {
        let mut p_buf = Vec::new();
        p.serialize(&mut p_buf)?;
        write_metadata_block_header(&mut out, false, 6, p_buf.len() as u32)?;
        out.write_all(&p_buf)?;
    }

    let mut loom_payload = Vec::new();
    header.serialize(&mut loom_payload)?;
    seek_table.serialize(&mut loom_payload)?;
    if let Some(eb) = edits {
        eb.serialize(&mut loom_payload)?;
    } else {
        let empty_edits = crate::container::edit_block::EditBlock {
            tracks_edits: std::collections::HashMap::new(),
        };
        empty_edits.serialize(&mut loom_payload)?;
    }
    loom_payload.extend_from_slice(&loom_frames_buf);

    let chunk_size = 16_000_000;
    for chunk in loom_payload.chunks(chunk_size) {
        write_metadata_block_header(&mut out, false, 2, chunk.len() as u32 + 4)?;
        out.write_all(b"LOOM")?;
        out.write_all(chunk)?;
    }

    write_metadata_block_header(&mut out, true, 1, 4096)?;
    out.write_all(&vec![0u8; 4096])?;

    out.write_all(&track0_frames_buf)?;
    Ok(out)
}

pub fn decode_session(session_bytes: &[u8]) -> io::Result<(Vec<Vec<Vec<i64>>>, SessionHeader)> {
    let (tracks, header, _, _, _) = decode_session_full(session_bytes)?;
    Ok((tracks, header))
}

pub fn decode_session_full(
    session_bytes: &[u8],
) -> io::Result<(
    Vec<Vec<Vec<i64>>>,
    SessionHeader,
    Option<EditBlock>,
    Option<MetadataTags>,
    Option<PictureBlock>,
)> {
    let mut cursor = Cursor::new(session_bytes);
    let mut magic = [0u8; 4];
    cursor.read_exact(&mut magic)?;

    let is_old_format = magic == *b"LSE\x01" || magic == *b"LOOM";
    if &magic != b"fLaC" && !is_old_format {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Not a FLAC or Loom file",
        ));
    }

    let mut loom_payload = Vec::new();
    let mut metadata_tags = None;
    let mut picture_block = None;

    if is_old_format {
        loom_payload.extend_from_slice(session_bytes);
    } else {
        loop {
            let mut header = [0u8; 4];
            cursor.read_exact(&mut header)?;
            let is_last = (header[0] & 0x80) != 0;
            let block_type = header[0] & 0x7F;
            let length = u32::from_be_bytes([0, header[1], header[2], header[3]]) as usize;

            if block_type == 4 {
                let mut data = vec![0u8; length];
                cursor.read_exact(&mut data)?;
                let mut r = std::io::Cursor::new(data);
                if let Ok(tags) = MetadataTags::deserialize(&mut r) {
                    metadata_tags = Some(tags);
                }
                if is_last {
                    break;
                }
                continue;
            } else if block_type == 6 {
                let mut data = vec![0u8; length];
                cursor.read_exact(&mut data)?;
                let mut r = std::io::Cursor::new(data);
                if let Ok(pic) = PictureBlock::deserialize(&mut r) {
                    picture_block = Some(pic);
                }
                if is_last {
                    break;
                }
                continue;
            } else if block_type == 2 {
                let mut app_id = [0u8; 4];
                cursor.read_exact(&mut app_id)?;
                if &app_id == b"LOOM" {
                    let mut data = vec![0u8; length - 4];
                    cursor.read_exact(&mut data)?;
                    loom_payload.extend_from_slice(&data);
                    if is_last {
                        break;
                    }
                    continue;
                } else {
                    cursor.set_position(cursor.position() + length as u64 - 4);
                }
            } else {
                cursor.set_position(cursor.position() + length as u64);
            }

            if is_last {
                break;
            }
        }
    }

    let track0_frames_offset = cursor.position() as usize;
    let track0_payload = &session_bytes[track0_frames_offset..];

    let mut loom_cursor = Cursor::new(&loom_payload);
    let header = SessionHeader::deserialize(&mut loom_cursor)?;
    let num_tracks = header.tracks.len();
    let bps = header.bit_depth as usize;

    let _seek_table = SeekTable::deserialize(&mut loom_cursor)?;
    let edit_block = match EditBlock::deserialize(&mut loom_cursor) {
        Ok(eb) => Some(eb),
        Err(_) => None,
    };

    let loom_frames_payload = &loom_payload[loom_cursor.position() as usize..];

    let mut out_tracks = vec![Vec::new(); num_tracks];
    let mut current_samples = vec![0u64; num_tracks];

    let mut ref_residuals: Option<Vec<i64>> = None;
    let mut reader0 = BitReader::new(track0_payload);
    let mut loom_pos = 0;

    let mut track0_block_size = 512;

    while current_samples[0] < header.tracks[0].total_samples {
        for t in 0..num_tracks {
            let (frame, _) = if t == 0 && !is_old_format {
                let f = Frame::deserialize_flac(&mut reader0, bps, &mut [])?;
                track0_block_size = f.block_size;
                (f, 0)
            } else {
                let mut length_buf = [0u8; 4];
                length_buf.copy_from_slice(&loom_frames_payload[loom_pos..loom_pos + 4]);
                let length = u32::from_be_bytes(length_buf) as usize;
                let start_loom_pos = loom_pos;
                loom_pos += 4;

                let mut reader_loom =
                    BitReader::new(&loom_frames_payload[loom_pos..loom_pos + length]);
                match Frame::deserialize_loom(&mut reader_loom, bps, &mut []) {
                    Ok((f, _)) => {
                        loom_pos += length;
                        (f, start_loom_pos)
                    }
                    Err(_) => {
                        let mut found_sync = false;
                        for i in (start_loom_pos + 4)..(loom_frames_payload.len() - 1) {
                            if loom_frames_payload[i] == 0xF8 && loom_frames_payload[i + 1] == 0xA5 {
                                loom_pos = i - 4;
                                found_sync = true;
                                break;
                            }
                        }
                        if !found_sync {
                            loom_pos = loom_frames_payload.len();
                        }

                        let channels_count = if !out_tracks[t].is_empty() {
                            out_tracks[t].len()
                        } else if loom_pos < loom_frames_payload.len() {
                            let mut next_length_buf = [0u8; 4];
                            next_length_buf.copy_from_slice(&loom_frames_payload[loom_pos..loom_pos + 4]);
                            let next_length = u32::from_be_bytes(next_length_buf) as usize;
                            let mut next_reader_loom =
                                BitReader::new(&loom_frames_payload[loom_pos + 4..loom_pos + 4 + next_length]);
                            if let Ok((f, _)) = Frame::deserialize_loom(&mut next_reader_loom, bps, &mut []) {
                                f.channel_layout.channels() as usize
                            } else {
                                1
                            }
                        } else {
                            1
                        };

                        use crate::container::frame::ChannelLayout;
                        let dummy_layout = match channels_count {
                            1 => ChannelLayout::Mono,
                            2 => ChannelLayout::Stereo,
                            3 => ChannelLayout::Surround3_0,
                            4 => ChannelLayout::Quad,
                            5 => ChannelLayout::Surround5_0,
                            6 => ChannelLayout::Surround5_1,
                            7 => ChannelLayout::Surround7_0,
                            8 => ChannelLayout::Surround7_1,
                            _ => ChannelLayout::Mono,
                        };

                        let dummy_subframes = vec![
                            crate::container::frame::Subframe {
                                mode: crate::predict::PredictionMode::Constant(0),
                                ref_track: None,
                                ref_weight: 0,
                                wasted_bits: 0,
                            };
                            channels_count
                        ];

                        let f = Frame {
                            frame_seq: 0,
                            block_size: track0_block_size,
                            channel_layout: dummy_layout,
                            subframes: dummy_subframes,
                        };
                        (f, start_loom_pos)
                    }
                }
            };

            let channels = frame.channel_layout.channels() as usize;
            if out_tracks[t].is_empty() {
                out_tracks[t] = vec![Vec::new(); channels];
            }

            let mut ch_residuals = vec![vec![0i64; frame.block_size as usize]; channels];

            for ch in 0..channels {
                match &frame.subframes[ch].mode {
                    PredictionMode::Constant(val) => ch_residuals[ch].fill(*val),
                    PredictionMode::Verbatim(samples) => ch_residuals[ch].copy_from_slice(samples),
                    PredictionMode::Fixed { residuals, .. } => {
                        ch_residuals[ch].copy_from_slice(residuals)
                    }
                    PredictionMode::Lpc { residuals, .. } => {
                        ch_residuals[ch].copy_from_slice(residuals)
                    }
                }
            }

            if t > 0 {
                if let Some(ref_res) = &ref_residuals {
                    if let Some(0) = frame.subframes[0].ref_track {
                        reconstruct_cross_prediction(
                            &mut ch_residuals[0],
                            ref_res,
                            frame.subframes[0].ref_weight,
                        );
                    }
                }
            }

            if t == 0 {
                ref_residuals = Some(ch_residuals[0].clone());
            }

            let mut ch_samples = vec![vec![0i64; frame.block_size as usize]; channels];
            for ch in 0..channels {
                let subframe = &frame.subframes[ch];
                match &subframe.mode {
                    PredictionMode::Constant(_) | PredictionMode::Verbatim(_) => {
                        ch_samples[ch].copy_from_slice(&ch_residuals[ch]);
                    }
                    _ => match &subframe.mode {
                        PredictionMode::Fixed { order, .. } => {
                            crate::predict::fixed::reconstruct_fixed(
                                &ch_residuals[ch],
                                &mut ch_samples[ch],
                                *order,
                            );
                        }
                        PredictionMode::Lpc {
                            order,
                            qlp_coeffs,
                            qlp_shift,
                            ..
                        } => {
                            crate::predict::lpc::reconstruct_lpc(
                                &ch_residuals[ch],
                                qlp_coeffs,
                                *qlp_shift,
                                *order,
                                &mut ch_samples[ch],
                            );
                        }
                        _ => {}
                    },
                }
                if subframe.wasted_bits > 0 {
                    for x in ch_samples[ch].iter_mut() {
                        *x <<= subframe.wasted_bits;
                    }
                }
            }

            let mut block_pcm = if channels == 2 {
                use crate::container::frame::ChannelLayout;
                let stereo_mode = match frame.channel_layout {
                    ChannelLayout::StereoLeftSide => StereoMode::LeftSide,
                    ChannelLayout::StereoRightSide => StereoMode::RightSide,
                    ChannelLayout::StereoMidSide => StereoMode::MidSide,
                    _ => StereoMode::Independent,
                };
                let (left, right) = reconstruct_stereo(&ch_samples[0], &ch_samples[1], stereo_mode);
                vec![left, right]
            } else {
                ch_samples
            };

            if let Some(eb) = &edit_block {
                if let Some(track_edits) = eb.tracks_edits.get(&(t as u16)) {
                    apply_edits(&mut block_pcm, current_samples[t], track_edits);
                }
            }

            for ch in 0..channels {
                out_tracks[t][ch].extend_from_slice(&block_pcm[ch]);
            }

            current_samples[t] += frame.block_size as u64;
        }
    }

    Ok((out_tracks, header, edit_block, metadata_tags, picture_block))
}
