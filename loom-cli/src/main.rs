use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use hound::{WavReader, WavSpec, WavWriter};
use loom_core::container::header::SessionHeader;
use loom_core::verify::{compute_pcm_md5, verify_stream};
use loom_core::{decode_session, decode_track, encode_session, encode_track};
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::Path;
use symphonia::core::audio::Signal;

#[derive(Parser)]
#[command(name = "loom")]
#[command(about = "Loom - A Session-Aware Lossless Audio Codec")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Encode {
        input: String,
        output: String,
        #[arg(long, default_value_t = 4096)]
        block_size: u32,
        #[arg(long)]
        thumbnail: Option<String>,
    },
    Decode {
        input: String,
        output: String,
    },
    Verify {
        input: String,
    },
    EncodeSession {
        input_dir: String,
        output: String,
        #[arg(long, default_value_t = 4096)]
        block_size: u32,
    },
    DecodeSession {
        input: String,
        output_dir: String,
    },
    Extract {
        input: String,
        track: String,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        output: String,
    },
    Edit {
        input: String,
        #[arg(long)]
        track: String,
        #[arg(long)]
        mute: Option<String>,
        #[arg(long)]
        fade_in: Option<String>,
        #[arg(long)]
        fade_out: Option<String>,
        #[arg(long)]
        gain: Option<String>,
    },
    Render {
        input: String,
        output: String,
    },
    Diff {
        v1: String,
        v2: String,
        output: String,
    },
    ApplyDiff {
        v1: String,
        diff: String,
        output: String,
    },
    Play {
        input: String,
    },

    Tag {
        input: String,

        #[arg(long, value_name = "KEY=VALUE")]
        set: Vec<String>,
    },
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Encode {
            input,
            output,
            block_size,
            thumbnail,
        } => {
            println!("Encoding {} to {}...", input, output);
            let input_path = Path::new(&input);
            let (channels, sample_rate, bit_depth) = read_audio_file(input_path)
                .map_err(|e| anyhow!("Failed to read audio file: {}", e))?;

            if channels.is_empty() {
                return Err(anyhow!("No audio channels decoded"));
            }

            let track_name = input_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("track");

            let mut picture_block = None;
            if let Some(thumb_path) = thumbnail {
                let bytes = std::fs::read(&thumb_path)
                    .map_err(|e| anyhow::anyhow!("Failed to read thumbnail: {}", e))?;

                picture_block = Some(loom_core::PictureBlock {
                    picture_type: loom_core::PictureType::FrontCover,
                    mime_type: if thumb_path.ends_with(".png") {
                        "image/png".to_string()
                    } else {
                        "image/jpeg".to_string()
                    },
                    description: "Cover".to_string(),
                    width: 0,
                    height: 0,
                    color_depth: 24,
                    num_colors: 0,
                    data: bytes,
                });
            }

            let compressed = loom_core::encode_session(
                &[channels],
                &[track_name.to_string()],
                sample_rate,
                bit_depth,
                block_size as usize,
                None,
                None,
                picture_block.as_ref(),
            )
            .map_err(|e| anyhow::anyhow!("Compression failed: {}", e))?;

            let mut out_file = File::create(&output)
                .map_err(|e| anyhow!("Failed to create output file: {}", e))?;
            out_file.write_all(&compressed)?;
            println!(
                "Encoding complete. Compressed size: {} bytes",
                compressed.len()
            );
        }
        Commands::Decode { input, output } => {
            let mut in_file = File::open(&input)?;
            let mut compressed = Vec::new();
            in_file.read_to_end(&mut compressed)?;

            let (tracks, header) =
                decode_session(&compressed).map_err(|e| anyhow!("Decompression failed: {}", e))?;
            let pcm_channels = &tracks[0];

            let num_channels = pcm_channels.len();
            let total_samples = pcm_channels[0].len();

            let mut interleaved = Vec::with_capacity(total_samples * num_channels);
            for s in 0..total_samples {
                for ch in 0..num_channels {
                    interleaved.push(pcm_channels[ch][s] as i32);
                }
            }

            if output == "-" {
                eprintln!(
                    "Streaming raw PCM to stdout ({} Hz, {} ch, {} bit)",
                    header.sample_rate, num_channels, header.bit_depth
                );
                let stdout = std::io::stdout();
                let mut out = stdout.lock();
                let bytes_per_sample = (header.bit_depth as usize + 7) / 8;
                for &sample in &interleaved {
                    let raw = sample.to_le_bytes();
                    out.write_all(&raw[..bytes_per_sample])?;
                }
                out.flush()?;
            } else {
                eprintln!("Decoding {} to {}...", input, output);
                let spec = WavSpec {
                    channels: num_channels as u16,
                    sample_rate: header.sample_rate,
                    bits_per_sample: header.bit_depth as u16,
                    sample_format: hound::SampleFormat::Int,
                };

                let mut writer = WavWriter::create(&output, spec)
                    .map_err(|e| anyhow!("Failed to create WAV writer: {}", e))?;
                for &sample in &interleaved {
                    writer.write_sample(sample)?;
                }
                writer.finalize()?;
                println!("Decoding complete.");
            }
        }
        Commands::Verify { input } => {
            println!("Verifying {}...", input);
            let mut in_file = File::open(&input)?;
            let mut compressed = Vec::new();
            in_file.read_to_end(&mut compressed)?;

            let (tracks, header) = decode_session(&compressed)
                .map_err(|e| anyhow!("Failed to decode file for verification: {}", e))?;
            let pcm_channels = &tracks[0];

            let num_channels = pcm_channels.len();
            let total_samples = pcm_channels[0].len();
            let mut interleaved = Vec::with_capacity(total_samples * num_channels);
            for s in 0..total_samples {
                for ch in 0..num_channels {
                    interleaved.push(pcm_channels[ch][s]);
                }
            }

            let computed = compute_pcm_md5(&interleaved, header.bit_depth);
            let stored = header.tracks[0].md5;

            if verify_stream(&computed, &stored) {
                println!("SUCCESS: Stream checksum is valid.");
            } else {
                println!("ERROR: Checksum mismatch!");
                println!("  Computed: {:x?}", computed);
                println!("  Stored:   {:x?}", stored);
                std::process::exit(1);
            }
        }
        Commands::EncodeSession {
            input_dir,
            output,
            block_size,
        } => {
            println!("Scanning stems in {}...", input_dir);
            let mut wav_paths = Vec::new();
            for entry in fs::read_dir(&input_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "wav") {
                    wav_paths.push(path);
                }
            }

            if wav_paths.is_empty() {
                return Err(anyhow!("No WAV files found in directory: {}", input_dir));
            }

            wav_paths.sort();

            let mut tracks = Vec::new();
            let mut track_names = Vec::new();
            let mut common_spec: Option<WavSpec> = None;
            let mut total_samples: Option<usize> = None;

            for path in &wav_paths {
                let mut reader = WavReader::open(path)?;
                let spec = reader.spec();

                if spec.sample_format != hound::SampleFormat::Int {
                    return Err(anyhow!(
                        "File {:?} has unsupported sample format (must be Integer PCM)",
                        path
                    ));
                }

                if let Some(common) = common_spec {
                    if spec.sample_rate != common.sample_rate
                        || spec.bits_per_sample != common.bits_per_sample
                    {
                        return Err(anyhow!(
                            "File {:?} properties mismatch with other stems (all stems must have identical sample rate and bit depth)",
                            path
                        ));
                    }
                } else {
                    common_spec = Some(spec);
                }

                let raw_samples: Vec<i64> = reader
                    .samples::<i32>()
                    .map(|s| s.map(|x| x as i64))
                    .collect::<Result<Vec<_>, _>>()?;

                let num_channels = spec.channels as usize;
                let file_samples = raw_samples.len() / num_channels;

                if let Some(common_len) = total_samples {
                    if file_samples != common_len {
                        return Err(anyhow!(
                            "File {:?} length mismatch (all stems must have identical sample counts)",
                            path
                        ));
                    }
                } else {
                    total_samples = Some(file_samples);
                }

                let mut track_channels = vec![vec![0i64; file_samples]; num_channels];
                for (i, &sample) in raw_samples.iter().enumerate() {
                    let ch = i % num_channels;
                    let idx = i / num_channels;
                    track_channels[ch][idx] = sample;
                }

                let stem_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("track")
                    .to_string();

                tracks.push(track_channels);
                track_names.push(stem_name);
            }

            let spec = common_spec.unwrap();
            println!(
                "Encoding session: {} tracks, {} Hz, {} bits, block size {}",
                tracks.len(),
                spec.sample_rate,
                spec.bits_per_sample,
                block_size
            );

            let compressed = encode_session(
                &tracks,
                &track_names,
                spec.sample_rate,
                spec.bits_per_sample as u8,
                block_size as usize,
                None,
                None,
                None,
            )
            .map_err(|e| anyhow!("Session compression failed: {}", e))?;

            let mut out_file = File::create(&output)?;
            out_file.write_all(&compressed)?;
            println!(
                "Session encoding complete. Compressed size: {} bytes",
                compressed.len()
            );
        }
        Commands::DecodeSession { input, output_dir } => {
            println!("Decoding session {} to {}...", input, output_dir);
            let mut in_file = File::open(&input)?;
            let mut compressed = Vec::new();
            in_file.read_to_end(&mut compressed)?;

            let (tracks, header) = decode_session(&compressed)
                .map_err(|e| anyhow!("Session decompression failed: {}", e))?;

            fs::create_dir_all(&output_dir)?;

            for (t, pcm_channels) in tracks.iter().enumerate() {
                let track_info = &header.tracks[t];
                let num_channels = pcm_channels.len();
                let total_samples = pcm_channels[0].len();

                let mut interleaved = Vec::with_capacity(total_samples * num_channels);
                for s in 0..total_samples {
                    for ch in 0..num_channels {
                        interleaved.push(pcm_channels[ch][s] as i32);
                    }
                }

                let out_wav_path = Path::new(&output_dir).join(format!("{}.wav", track_info.name));

                let spec = WavSpec {
                    channels: num_channels as u16,
                    sample_rate: header.sample_rate,
                    bits_per_sample: header.bit_depth as u16,
                    sample_format: hound::SampleFormat::Int,
                };

                let mut writer = WavWriter::create(&out_wav_path, spec).map_err(|e| {
                    anyhow!("Failed to create WAV writer for {:?}: {}", out_wav_path, e)
                })?;
                for &sample in &interleaved {
                    writer.write_sample(sample)?;
                }
                writer.finalize()?;
                println!("  Decoded track: {}", track_info.name);
            }

            println!("Session decoding complete.");
        }
        Commands::Extract {
            input,
            track,
            from,
            to,
            output,
        } => {
            println!(
                "Extracting track '{}' from range {} to {}...",
                track, from, to
            );
            let mut in_file = File::open(&input)?;
            let mut compressed = Vec::new();
            in_file.read_to_end(&mut compressed)?;

            let mut cursor = Cursor::new(&compressed);
            let header = SessionHeader::deserialize(&mut cursor)?;
            let track_idx = header
                .tracks
                .iter()
                .position(|t| t.name == track)
                .ok_or_else(|| anyhow!("Track '{}' not found in session header", track))?;

            let sample_rate = header.sample_rate;
            let start_sample = parse_time_or_samples(&from, sample_rate)
                .map_err(|e| anyhow!("Failed to parse 'from' offset: {}", e))?;
            let end_sample = parse_time_or_samples(&to, sample_rate)
                .map_err(|e| anyhow!("Failed to parse 'to' offset: {}", e))?;

            let (pcm_channels, _) = loom_core::decoder::decode_track_partial(
                &compressed,
                track_idx,
                start_sample as u64,
                (end_sample - start_sample) as usize,
            )
            .map_err(|e| anyhow!("Range extraction failed: {}", e))?;

            let num_channels = pcm_channels.len();
            let total_samples = pcm_channels[0].len();

            let mut interleaved = Vec::with_capacity(total_samples * num_channels);
            for s in 0..total_samples {
                for ch in 0..num_channels {
                    interleaved.push(pcm_channels[ch][s] as i32);
                }
            }

            let spec = WavSpec {
                channels: num_channels as u16,
                sample_rate,
                bits_per_sample: header.bit_depth as u16,
                sample_format: hound::SampleFormat::Int,
            };

            let mut writer = WavWriter::create(&output, spec)
                .map_err(|e| anyhow!("Failed to create WAV writer: {}", e))?;
            for &sample in &interleaved {
                writer.write_sample(sample)?;
            }
            writer.finalize()?;
            println!(
                "Extraction complete. Saved {} samples to {}",
                total_samples, output
            );
        }
        Commands::Edit {
            input,
            track,
            mute,
            fade_in,
            fade_out,
            gain,
        } => {
            println!("Editing track '{}' in session {}...", track, input);
            let mut in_file = File::open(&input)?;
            let mut compressed = Vec::new();
            in_file.read_to_end(&mut compressed)?;

            let mut cursor = Cursor::new(&compressed);
            let header = SessionHeader::deserialize(&mut cursor)?;
            let track_idx = header
                .tracks
                .iter()
                .position(|t| t.name == track)
                .ok_or_else(|| anyhow!("Track '{}' not found in session", track))?
                as u16;

            let sample_rate = header.sample_rate;
            let mut edit_block = get_edit_block(&compressed)?;

            let track_edits = edit_block
                .tracks_edits
                .entry(track_idx)
                .or_insert_with(loom_core::TrackEdits::new);

            if let Some(m) = mute {
                let (start, end) = parse_range(&m, sample_rate)?;
                track_edits.mutes.push(loom_core::MuteRegion {
                    start_sample: start,
                    end_sample: end,
                });
                println!("  Added mute region: {} to {} samples", start, end);
            }

            if let Some(fi) = fade_in {
                let (start, end) = parse_range(&fi, sample_rate)?;
                track_edits.fades.push(loom_core::Fade {
                    start_sample: start,
                    end_sample: end,
                    shape: loom_core::FadeShape::Linear,
                    is_fade_in: true,
                });
                println!("  Added fade in: {} to {} samples", start, end);
            }

            if let Some(fo) = fade_out {
                let (start, end) = parse_range(&fo, sample_rate)?;
                track_edits.fades.push(loom_core::Fade {
                    start_sample: start,
                    end_sample: end,
                    shape: loom_core::FadeShape::Linear,
                    is_fade_in: false,
                });
                println!("  Added fade out: {} to {} samples", start, end);
            }

            if let Some(g) = gain {
                let (offset, val) = parse_gain_point(&g, sample_rate)?;
                track_edits.gain_points.push(loom_core::GainPoint {
                    sample_offset: offset,
                    gain: val,
                });

                track_edits.gain_points.sort_by_key(|p| p.sample_offset);
                println!(
                    "  Added gain point: sample {} -> multiplier {}",
                    offset, val
                );
            }

            let (tracks, header, _, tags, picture) = loom_core::decode_session_full(&compressed)?;
            let mut track_names = Vec::new();
            for t in &header.tracks {
                track_names.push(t.name.clone());
            }
            let updated = loom_core::encode_session(
                &tracks,
                &track_names,
                header.sample_rate,
                header.bit_depth,
                4096,
                Some(&edit_block),
                tags.as_ref(),
                picture.as_ref(),
            )?;
            let mut out_file = File::create(&input)?;
            out_file.write_all(&updated)?;
            println!("Metadata update complete (non-destructive).");
        }
        Commands::Render { input, output } => {
            println!("Rendering session {} to mixed output {}...", input, output);
            let mut in_file = File::open(&input)?;
            let mut compressed = Vec::new();
            in_file.read_to_end(&mut compressed)?;

            let (tracks, header) = decode_session(&compressed)
                .map_err(|e| anyhow!("Failed to decode session: {}", e))?;

            if tracks.is_empty() {
                return Err(anyhow!("No tracks to render"));
            }

            let num_tracks = tracks.len();
            let total_samples = tracks[0][0].len();

            let mut mix_channels = 1;
            for t in &tracks {
                if t.len() == 2 {
                    mix_channels = 2;
                }
            }

            let mut mixed = vec![vec![0i64; total_samples]; mix_channels];
            for s in 0..total_samples {
                for ch in 0..mix_channels {
                    let mut sum = 0i64;
                    for t in 0..num_tracks {
                        let track_ch_count = tracks[t].len();
                        let sample_val = if track_ch_count == 2 {
                            tracks[t][ch][s]
                        } else {
                            tracks[t][0][s]
                        };
                        sum += sample_val;
                    }

                    let limit = match header.bit_depth {
                        16 => 32768,
                        24 => 8388608,
                        _ => 2147483648,
                    };
                    mixed[ch][s] = sum.clamp(-limit, limit - 1);
                }
            }

            let mut interleaved = Vec::with_capacity(total_samples * mix_channels);
            for s in 0..total_samples {
                for ch in 0..mix_channels {
                    interleaved.push(mixed[ch][s] as i32);
                }
            }

            let spec = WavSpec {
                channels: mix_channels as u16,
                sample_rate: header.sample_rate,
                bits_per_sample: header.bit_depth as u16,
                sample_format: hound::SampleFormat::Int,
            };

            let mut writer = WavWriter::create(&output, spec)
                .map_err(|e| anyhow!("Failed to create WAV writer: {}", e))?;
            for &sample in &interleaved {
                writer.write_sample(sample)?;
            }
            writer.finalize()?;
            println!(
                "Rendering complete. Mixed {} tracks to {}",
                num_tracks, output
            );
        }
        Commands::Diff { v1, v2, output } => {
            println!("Computing diff: base {} -> target {}...", v1, v2);
            let mut v1_file = File::open(&v1)?;
            let mut v1_bytes = Vec::new();
            v1_file.read_to_end(&mut v1_bytes)?;

            let mut v2_file = File::open(&v2)?;
            let mut v2_bytes = Vec::new();
            v2_file.read_to_end(&mut v2_bytes)?;

            let diff = loom_core::encode_diff(&v1_bytes, &v2_bytes)
                .map_err(|e| anyhow!("Failed to compute diff: {}", e))?;

            let mut out_file = File::create(&output)?;
            diff.serialize(&mut out_file)?;
            println!("Diff created successfully. Saved to {}", output);
        }
        Commands::ApplyDiff { v1, diff, output } => {
            println!("Applying diff {} to base {}...", diff, v1);
            let mut v1_file = File::open(&v1)?;
            let mut v1_bytes = Vec::new();
            v1_file.read_to_end(&mut v1_bytes)?;

            let mut diff_file = File::open(&diff)?;
            let mut diff_bytes = Vec::new();
            diff_file.read_to_end(&mut diff_bytes)?;

            let mut cursor = Cursor::new(&diff_bytes);
            let parsed_diff = loom_core::SessionDiff::deserialize(&mut cursor)
                .map_err(|e| anyhow!("Failed to deserialize diff: {}", e))?;

            let reconstructed = loom_core::apply_diff(&v1_bytes, &parsed_diff)
                .map_err(|e| anyhow!("Failed to apply diff: {}", e))?;

            let mut out_file = File::create(&output)?;
            out_file.write_all(&reconstructed)?;
            println!("Reconstruction complete. Saved to {}", output);
        }
        Commands::Play { input } => {
            println!("Playing Loom file {}...", input);
            let mut in_file = File::open(&input)?;
            let mut compressed = Vec::new();
            in_file.read_to_end(&mut compressed)?;

            let is_session = compressed.starts_with(b"LSE\x01");

            let (interleaved_samples, sample_rate, num_channels) = if is_session {
                let (tracks, header) = decode_session(&compressed)
                    .map_err(|e| anyhow!("Failed to decode session: {}", e))?;

                if tracks.is_empty() {
                    return Err(anyhow!("No tracks to play"));
                }

                let total_samples = tracks[0][0].len();
                let mut mix_channels = 1;
                for t in &tracks {
                    if t.len() == 2 {
                        mix_channels = 2;
                    }
                }

                let mut mixed = vec![vec![0i64; total_samples]; mix_channels];
                for s in 0..total_samples {
                    for ch in 0..mix_channels {
                        let mut sum = 0i64;
                        for t in 0..tracks.len() {
                            let track_ch_count = tracks[t].len();
                            let sample_val = if track_ch_count == 2 {
                                tracks[t][ch][s]
                            } else {
                                tracks[t][0][s]
                            };
                            sum += sample_val;
                        }
                        let limit = match header.bit_depth {
                            16 => 32768,
                            24 => 8388608,
                            _ => 2147483648,
                        };
                        mixed[ch][s] = sum.clamp(-limit, limit - 1);
                    }
                }

                let mut interleaved = Vec::with_capacity(total_samples * mix_channels);
                for s in 0..total_samples {
                    for ch in 0..mix_channels {
                        interleaved.push(mixed[ch][s] as i16);
                    }
                }
                (interleaved, header.sample_rate, mix_channels)
            } else {
                let (tracks, header) = loom_core::decode_session(&compressed)
                    .map_err(|e| anyhow!("Failed to decode track: {}", e))?;
                let pcm_channels = &tracks[0];

                if pcm_channels.is_empty() {
                    return Err(anyhow!("No channels to play"));
                }

                let num_channels = pcm_channels.len();
                let total_samples = pcm_channels[0].len();
                let mut interleaved = Vec::with_capacity(total_samples * num_channels);
                for s in 0..total_samples {
                    for ch in 0..num_channels {
                        interleaved.push(pcm_channels[ch][s] as i16);
                    }
                }
                (interleaved, header.sample_rate, num_channels)
            };

            println!(
                "Starting playback ({} Hz, {} channels)... Press Ctrl+C to stop.",
                sample_rate, num_channels
            );

            let (_stream, stream_handle) = rodio::OutputStream::try_default()
                .map_err(|e| anyhow!("Failed to open default audio output device: {}", e))?;
            let sink = rodio::Sink::try_new(&stream_handle)
                .map_err(|e| anyhow!("Failed to create audio playback sink: {}", e))?;

            let buffer = rodio::buffer::SamplesBuffer::new(
                num_channels as u16,
                sample_rate,
                interleaved_samples,
            );

            sink.append(buffer);
            sink.sleep_until_end();
            println!("Playback complete.");
        }
        Commands::Tag { input, set } => {
            let mut in_file = File::open(&input)?;
            let mut data = Vec::new();
            in_file.read_to_end(&mut data)?;

            if set.is_empty() {
                let is_session = data.starts_with(b"LSE\x01");
                if is_session {
                    let (_, _, _, tags_opt, _) = loom_core::decode_session_full(&data)
                        .map_err(|e| anyhow!("Failed to decode session: {}", e))?;
                    match tags_opt {
                        Some(tags) => {
                            if tags.tags.is_empty() {
                                println!("No metadata tags found.");
                            } else {
                                println!("Metadata tags:");
                                let mut sorted: Vec<_> = tags.tags.iter().collect();
                                sorted.sort_by_key(|(k, _)| k.to_lowercase());
                                for (k, v) in sorted {
                                    println!("  {} = {}", k, v);
                                }
                            }
                        }
                        None => println!("No metadata tags found."),
                    }
                } else {
                    println!("Note: Single-track files do not currently store metadata tags.");
                    println!("Encode as a session (encode-session) to use tags.");
                }
            } else {
                let is_session = data.starts_with(b"fLaC");
                if !is_session {
                    return Err(anyhow!("Tagging is only supported for session files (.loom encoded via encode-session). Use encode-session first."));
                }

                let (tracks, header, edits, existing_tags, picture) =
                    loom_core::decode_session_full(&data)
                        .map_err(|e| anyhow!("Failed to decode session: {}", e))?;
                let mut tags = existing_tags.unwrap_or_else(|| loom_core::MetadataTags::new());

                for kv in &set {
                    if let Some(eq_pos) = kv.find('=') {
                        let key = kv[..eq_pos].to_string();
                        let value = kv[eq_pos + 1..].to_string();
                        tags.tags.insert(key, value);
                    } else {
                        return Err(anyhow!("Invalid tag format: '{}'. Expected KEY=VALUE", kv));
                    }
                }

                let mut track_names = Vec::new();
                for t in &header.tracks {
                    track_names.push(t.name.clone());
                }

                let updated = loom_core::encode_session(
                    &tracks,
                    &track_names,
                    header.sample_rate,
                    header.bit_depth,
                    4096,
                    edits.as_ref(),
                    Some(&tags),
                    picture.as_ref(),
                )
                .map_err(|e| anyhow!("Failed to encode tags: {}", e))?;

                let mut out_file = File::create(&input)?;
                out_file.write_all(&updated)?;
                println!("Tags updated successfully.");
            }
        }
    }

    Ok(())
}

fn parse_time_or_samples(val: &str, sample_rate: u32) -> Result<u64> {
    if val.ends_with('s') {
        let secs: f64 = val[0..val.len() - 1].parse()?;
        Ok((secs * sample_rate as f64).round() as u64)
    } else {
        Ok(val.parse()?)
    }
}

fn parse_range(range_str: &str, sample_rate: u32) -> Result<(u64, u64)> {
    let parts: Vec<&str> = range_str.split('-').collect();
    if parts.len() != 2 {
        return Err(anyhow!(
            "Invalid range format (must be start-end, e.g. 0-2s)"
        ));
    }
    let start = parse_time_or_samples(parts[0], sample_rate)?;
    let end = parse_time_or_samples(parts[1], sample_rate)?;
    Ok((start, end))
}

fn parse_gain_point(point_str: &str, sample_rate: u32) -> Result<(u64, f32)> {
    let parts: Vec<&str> = point_str.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow!(
            "Invalid gain point format (must be time:multiplier, e.g. 5s:1.5)"
        ));
    }
    let offset = parse_time_or_samples(parts[0], sample_rate)?;
    let gain: f32 = parts[1].parse()?;
    Ok((offset, gain))
}

fn get_edit_block(session_bytes: &[u8]) -> Result<loom_core::EditBlock> {
    let mut cursor = Cursor::new(session_bytes);
    let _header = SessionHeader::deserialize(&mut cursor)?;

    let mut edit_block = loom_core::EditBlock::new();
    let mut term_buf = [0u8; 1];
    loop {
        cursor.read_exact(&mut term_buf)?;
        if term_buf[0] == 0xFF {
            break;
        }
        let block_type = term_buf[0];
        let mut len_buf = [0u8; 4];
        cursor.read_exact(&mut len_buf)?;
        let length = u32::from_be_bytes(len_buf) as usize;

        if block_type == 0x01 {
            edit_block = loom_core::EditBlock::deserialize(&mut cursor)?;
        } else {
            let pos = cursor.position();
            cursor.set_position(pos + length as u64);
        }
    }
    Ok(edit_block)
}

fn read_audio_file(path: &Path) -> Result<(Vec<Vec<i64>>, u32, u8)> {
    let src = File::open(path).map_err(|e| anyhow!("Failed to open file {:?}: {}", path, e))?;
    let mss = symphonia::core::io::MediaSourceStream::new(Box::new(src), Default::default());
    let mut hint = symphonia::core::probe::Hint::new();

    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &symphonia::core::formats::FormatOptions::default(),
            &symphonia::core::meta::MetadataOptions::default(),
        )
        .map_err(|e| anyhow!("Failed to probe audio formats: {}", e))?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .first()
        .ok_or_else(|| anyhow!("No tracks found in audio file"))?;

    let dec_opts = symphonia::core::codecs::DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .map_err(|e| anyhow!("Failed to initialize audio decoder: {}", e))?;
    let track_id = track.id;

    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| anyhow!("Unknown sample rate"))?;
    let bit_depth = track.codec_params.bits_per_sample.unwrap_or(16) as u8;

    let mut pcm_channels: Vec<Vec<i64>> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(ref err))
                if err.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(err) => return Err(anyhow!("Failed to read next packet: {}", err)),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = decoder
            .decode(&packet)
            .map_err(|e| anyhow!("Audio decoding failed: {}", e))?;

        match decoded {
            symphonia::core::audio::AudioBufferRef::F32(buf) => {
                if pcm_channels.is_empty() {
                    pcm_channels = vec![Vec::new(); buf.spec().channels.count()];
                }
                let max_val = ((1u64 << (bit_depth - 1)) - 1) as f64;
                let min_val = -(1i64 << (bit_depth - 1)) as f64;
                for ch in 0..buf.spec().channels.count() {
                    for &sample in buf.chan(ch) {
                        pcm_channels[ch]
                            .push((sample as f64 * max_val).round().clamp(min_val, max_val) as i64);
                    }
                }
            }
            symphonia::core::audio::AudioBufferRef::S16(buf) => {
                if pcm_channels.is_empty() {
                    pcm_channels = vec![Vec::new(); buf.spec().channels.count()];
                }
                for ch in 0..buf.spec().channels.count() {
                    for &sample in buf.chan(ch) {
                        pcm_channels[ch].push(sample as i64);
                    }
                }
            }
            symphonia::core::audio::AudioBufferRef::S32(buf) => {
                if pcm_channels.is_empty() {
                    pcm_channels = vec![Vec::new(); buf.spec().channels.count()];
                }
                for ch in 0..buf.spec().channels.count() {
                    for &sample in buf.chan(ch) {
                        pcm_channels[ch].push(sample as i64);
                    }
                }
            }
            symphonia::core::audio::AudioBufferRef::U8(buf) => {
                if pcm_channels.is_empty() {
                    pcm_channels = vec![Vec::new(); buf.spec().channels.count()];
                }
                for ch in 0..buf.spec().channels.count() {
                    for &sample in buf.chan(ch) {
                        pcm_channels[ch].push((sample as i16 - 128) as i64);
                    }
                }
            }
            _ => {
                return Err(anyhow!("Unsupported audio buffer format"));
            }
        }
    }

    Ok((pcm_channels, sample_rate, bit_depth))
}
