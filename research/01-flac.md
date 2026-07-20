# Research Note 01: FLAC Format Overview

**Source**: [RFC 9639](https://www.rfc-editor.org/rfc/rfc9639.html) (primary), [Old FLAC Format Spec](https://xiph.org/flac/old_format.html)

---

## Stream Structure

```
STREAM := "fLaC" METADATA_BLOCK+ FRAME+
```

- Magic bytes: `0x66 0x4C 0x61 0x43` ("fLaC")
- First METADATA_BLOCK is always STREAMINFO (type 0)
- Zero or more additional metadata blocks follow
- Audio frames follow metadata

### METADATA_BLOCK_HEADER (32 bits)
| Field | Bits | Meaning |
|-------|------|---------|
| last_block | 1 | 1 = this is the last metadata block |
| block_type | 7 | 0=STREAMINFO, 1=PADDING, 2=APPLICATION, 3=SEEKTABLE, 4=VORBIS_COMMENT, 5=CUESHEET, 6=PICTURE, 7-126=reserved, 127=invalid |
| length | 24 | Length of metadata payload in bytes |

### METADATA_BLOCK_STREAMINFO (34 bytes = 272 bits)
| Field | Bits | Meaning |
|-------|------|---------|
| min_block_size | 16 | Minimum blocksize in samples (≥ 16) |
| max_block_size | 16 | Maximum blocksize in samples (≤ 65535) |
| min_frame_size | 24 | Min frame size in bytes (0 = unknown) |
| max_frame_size | 24 | Max frame size in bytes (0 = unknown) |
| sample_rate | 20 | Hz, max 655350, nonzero |
| channels | 3 | (num_channels − 1), supports 1–8 channels |
| bits_per_sample | 5 | (bps − 1), supports 4–32 bps |
| total_samples | 36 | 0 = unknown |
| md5_signature | 128 | MD5 of unencoded PCM audio |

### SEEKTABLE
Each seekpoint is 18 bytes:
| Field | Bits | Meaning |
|-------|------|---------|
| sample_number | 64 | First sample of target frame; `0xFFFF...` = placeholder |
| byte_offset | 64 | Offset in bytes from first frame |
| frame_samples | 16 | Number of samples in target frame |

---

## Frame Structure

```
FRAME := FRAME_HEADER SUBFRAME+ <zero-padding to byte> FRAME_FOOTER
```

### FRAME_HEADER
| Field | Bits | Meaning |
|-------|------|---------|
| sync | 14 | `0b11111111111110` |
| reserved | 1 | Must be 0 |
| blocking_strategy | 1 | 0=fixed-blocksize (frame number encoded), 1=variable (sample number encoded) |
| block_size_code | 4 | Lookup table (see below) |
| sample_rate_code | 4 | Lookup table |
| channel_assignment | 4 | 0–7 = independent channels 1–8, 8 = left/side, 9 = right/side, 10 = mid/side |
| bits_per_sample | 3 | Lookup table |
| reserved | 1 | Must be 0 |
| frame/sample_number | 8–56 | UTF-8 coded integer |
| blocksize_ext | 0/8/16 | If block_size_code == 6 → 8-bit (blocksize-1); if == 7 → 16-bit |
| samplerate_ext | 0/8/16 | If rate_code indicates |
| CRC-8 | 8 | CRC of frame header |

**Block size lookup** (block_size_code):
- `0001` = 192
- `0010`–`0101` = 576 × 2^(n-2)
- `0110` = read 8-bit (blocksize-1) at end of header
- `0111` = read 16-bit (blocksize-1) at end of header
- `1000`–`1111` = 256 × 2^(n-8)

**Channel assignment** (bits 4–7 of byte 3):
- `0000`–`0111` = 1–8 independent channels
- `1000` = Left + Side (left/side stereo)
- `1001` = Right + Side (right/side stereo)
- `1010` = Mid + Side (mid/side stereo)

### FRAME_FOOTER
| Field | Bits | Meaning |
|-------|------|---------|
| CRC-16 | 16 | CRC of entire frame including header |

---

## Subframe Structure

```
SUBFRAME := SUBFRAME_HEADER SUBFRAME_DATA
```

### SUBFRAME_HEADER
| Field | Bits | Meaning |
|-------|------|---------|
| reserved | 1 | Must be 0 |
| subframe_type | 6 | `000000`=CONSTANT, `000001`=VERBATIM, `001000`–`001100`=FIXED(0-4), `100000`–`111111`=LPC(1-32) |
| wasted_bits_flag | 1 | 1 = next unary-coded value gives number of wasted bits |

**Subframe types:**
- `CONSTANT` (0): Single sample value; entire block has that value
- `VERBATIM` (1): Raw PCM, uncompressed
- `FIXED` (8–12): Fixed linear predictor of order 0–4
- `LPC` (32–63): Adaptive LPC of order 1–32

### SUBFRAME_FIXED
Contains:
1. `order` warm-up samples (bps bits each)
2. RESIDUAL (Rice-coded)

### SUBFRAME_LPC
Contains:
1. `order` warm-up samples (bps bits each)
2. LPC precision (4 bits, value+1 = actual precision 1–16)
3. LPC shift (signed 5 bits)
4. `order` quantized coefficients (precision bits each, signed)
5. RESIDUAL (Rice-coded)

---

## Residual / Rice Coding

```
RESIDUAL := coding_method(2) partition_order(4) RICE_PARTITION+
```

- `coding_method`: 0 = Rice, 1 = Rice2 (wider parameter)
- `partition_order`: 0–15; number of partitions = 2^partition_order
- First partition excludes warm-up samples

**RICE_PARTITION:**
- Rice parameter k (4 or 5 bits depending on coding_method)
- If k == `1111` (or `11111` for Rice2): escape-coded (raw bps bits per sample)
- Residual samples: for each sample `x`, encoded as unary quotient + binary remainder
  - Map signed → unsigned: `n >= 0 ? 2n : -2n-1` (zigzag/fold)
  - Encoded value `e = fold(x)`: write `e >> k` in unary (that many `0` bits then one `1`), then `e & ((1<<k)-1)` in binary

**Rice parameter selection**: FLAC chooses k for each partition to minimize total bits. Optimal k ≈ log2(mean(|residuals|)).

---

## Loom Design Decisions (from this research)

1. **Loom uses its own bitstream**, not FLAC's. We design an analogous structure:
   - Magic: `b"LOOM"` (4 bytes)
   - Session header block (analogous to STREAMINFO + session metadata)
   - Per-track seek index blocks
   - Per-track frame sequences
   - Edit metadata block (Phase 4)
   - Diff block (Phase 5)

2. **Same subframe type taxonomy** (Constant, Verbatim, Fixed-0..4, LPC) with same bit layout for predictors and residuals, but different container framing.

3. **Same Rice coding**: the entropy coding is unambiguously specified; we implement it identically.

4. **MD5 checksum** stored in the session header for each track.

---

## References

1. **RFC 9639 (2024):** *FLAC Audio Coding Format.* Internet Engineering Task Force (IETF). [https://www.rfc-editor.org/rfc/rfc9639.html](https://www.rfc-editor.org/rfc/rfc9639.html)
2. **Xiph.Org Foundation (2000):** *FLAC Format Specification (Legacy).* [https://xiph.org/flac/old_format.html](https://xiph.org/flac/old_format.html)
