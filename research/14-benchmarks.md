# Research Paper 12: Codec Evaluation Methodology: Comprehensive Benchmark Framework, Dataset Taxonomy, and Empirical Testing Infrastructure

**Author:** Loom Codec Research Group  
**Status:** Complete  
**Date:** July 2026  

---

## 1. Problem Statement

Evaluating lossless audio compression algorithms requires rigorous, multi-dimensional benchmarking. A common mistake in codec evaluation is measuring only a single metric (such as compression ratio) on a narrow or biased dataset (such as standard 16-bit stereo CD tracks).

In modern multitrack production and archiving workflows, a codec must balance multiple competing objectives:
1. **Compression Ratio ($CR$):** Minimizing storage footprint across diverse signal types (tonal, transient, noisy, synthetic, multitrack stems).
2. **Encoding Throughput ($\text{MB/s}$):** Fast compression for DAW rendering and real-time bounce operations.
3. **Decoding Throughput ($\text{MB/s}$):** Ultra-fast decompression for high-track-count DAW playhead playback.
4. **Random Access Latency ($\tau_{\text{seek}}$):** Instant $O(1)$ seeking latency for DAW scrub and zoom interactions.
5. **Memory Footprint ($\text{MB}$):** Deterministic RAM consumption during multitrack encoding/decoding.

This paper establishes the formal benchmarking framework for **Loom**, defining standardized audio corpora, mathematical evaluation formulas, execution harnesses, and reproducible comparison protocols against `libFLAC`, `FFmpeg FLAC`, `WavPack`, `ALAC`, `Monkey's Audio (APE)`, and `TAK`.

---

## 2. Historical Background

- **EBU SQAM (1988):** The European Broadcasting Union established the Sound Quality Assessment Material corpus for evaluating digital audio systems.
- **Shorten vs. FLAC Benchmarks (1994–2000):** Early lossless codec evaluations focused primarily on compression ratio and decode speed on 16-bit 44.1kHz stereo audio.
- **Hydrogenaudio Lossless Comparison (2004–2015):** The community established standard multi-genre audio test suites, benchmarking FLAC, WavPack, APE, TAK, and ALAC across speed presets (-0 through -8).
- **Loom Multitrack Session Corpus (2026):** Loom expands evaluation to multitrack DAW project sessions, measuring cross-track entropy reduction, edit metadata update overheads, and localized version diffing.

---

## 3. Mathematical Evaluation Metrics

### 3.1 Compression Ratio ($CR$) and Space Savings ($SS$)

Let $S_{\text{raw}}$ be the total uncompressed PCM size in bytes, and $S_{\text{comp}}$ be the compressed bitstream size in bytes.

$$\text{Compression Ratio } (CR) = \frac{S_{\text{raw}}}{S_{\text{comp}}}$$

$$\text{Space Savings } (SS) = \left( 1 - \frac{S_{\text{comp}}}{S_{\text{raw}}} \right) \times 100\%$$

### 3.2 Bits Per Sample ($BPS_{\text{compressed}}$)

For an audio signal with bit depth $b$ (e.g., $b = 24$), total channels $C$, and total frame samples $N_{\text{total}}$:

$$BPS_{\text{compressed}} = \frac{8 \times S_{\text{comp}}}{C \times N_{\text{total}}}$$

### 3.3 Throughput Metrics

Let $T_{\text{enc}}$ be the total encoding wall-clock time in seconds across $C$ channels of $N_{\text{total}}$ samples at sample rate $f_s$:

$$\text{Encode Speed Ratio} = \frac{N_{\text{total}} / f_s}{T_{\text{enc}}} \quad (\text{x Real-Time})$$

$$\text{Encoding Throughput} = \frac{S_{\text{raw}}}{10^6 \times T_{\text{enc}}} \quad (\text{MB/s})$$

### 3.4 Random Access Seek Latency ($\tau_{\text{seek}}$)

For a target sample timestamp $t_{\text{target}}$:
$$\tau_{\text{seek}} = T_{\text{index\_lookup}} + T_{\text{frame\_read}} + T_{\text{decode\_block}}$$

---

## 4. Dataset Taxonomy

The Loom evaluation suite uses five standardized audio corpora covering diverse signal characteristics:

```
                                Loom Audio Corpus
                                        |
       +-----------------+--------------+---------------+-----------------+
       |                 |              |               |                 |
       v                 v              v               v                 v
Corpus A: Solo/Tonal   Corpus B: Dense  Corpus C: High   Corpus D: Multi-  Corpus E: Edits
(Flute, Harpsichord,   Rock/Electronic  Res 24/96        track DAW Stems   & Revisions
Pitched Vocals)        (Percussion,     Orchestral /     (24–64 Parallel   (Version Punch-
                       Distortion)      Acoustic         Tracks)           ins, Fades)
```

1. **Corpus A (Solo / Tonal Instruments):** High autocorrelation, stationary pitch. Evaluates high-order adaptive LPC performance.
2. **Corpus B (Dense / Transient Material):** Sharp attacks, broadband noise floors, drums. Evaluates fixed predictor switching and Rice parameter selection.
3. **Corpus C (High-Resolution Audiophile):** 24-bit / 96kHz and 24-bit / 192kHz multi-mic recordings. Evaluates 64-bit integer MAC accuracy and wasted bits handling.
4. **Corpus D (Multitrack DAW Sessions):** Raw unmixed multitrack stem folders (24 to 64 stems). Evaluates Loom's Cross-Track Prediction engine.
5. **Corpus E (Session Revisions):** Successive project mixes ($v_1, v_2, v_3$) with localized punch-in edits. Evaluates Loom Frame-Level Version Diffing (`loom diff`).

---

## 5. Complexity & Execution Environment

To ensure reproducible measurements, benchmarks are executed inside an isolated test environment:
- **Hardware Platform:** Intel Core i9-13900K / Apple M3 Max.
- **OS Environment:** Linux 6.8 kernel / macOS Sonoma.
- **Process Isolation:** Fixed CPU frequency affinity (`taskset`), CPU governor set to `performance`, disabled hyperthreading during single-core runs.
- **Memory Profiling:** Valgrind Massif / Heaptrack for peak memory usage ($\text{MB}$).

---

## 6. Comparison Protocol

| Codec | Binary / Version | Command Flags | Target Profile |
| :--- | :--- | :--- | :--- |
| **libFLAC** | `flac 1.4.3` | `-0` (Fastest), `-5` (Default), `-8` (Best) | FLAC RFC 9639 Baseline |
| **FFmpeg FLAC** | `ffmpeg 6.1` | `-c:a flac -compression_level 12` | Alternate FLAC implementation |
| **WavPack** | `wavpack 5.6.0` | `-fast`, `-default`, `-hh` (High) | Multichannel comparison |
| **ALAC** | `refALAC` | Default | Apple ecosystem baseline |
| **Monkey's Audio** | `mac 10.12` | `-c2000` (Extra High) | Maximum compression baseline |
| **TAK** | `takc 2.3.0` | `-p2` (Default), `-p4` (Max) | High-speed compression baseline |
| **Loom** | `loom-cli 0.1.0` | `encode`, `encode-session` | Proposed Codec |

---

## 7. Implementation Strategy: Rust Automated Benchmark Rig

Loom integrates a custom benchmarking harness built on Criterion.rs and dedicated CLI commands (`loom benchmark`, `loom analyze`, `loom compare`):

```rust
use std::time::Instant;

pub struct BenchmarkResult {
    pub dataset_name: String,
    pub raw_bytes: u64,
    pub compressed_bytes: u64,
    pub compression_ratio: f64,
    pub encode_time_secs: f64,
    pub decode_time_secs: f64,
    pub encode_mb_per_sec: f64,
    pub decode_mb_per_sec: f64,
}

pub fn run_codec_benchmark(
    samples: &[Vec<Vec<i64>>],
    sample_rate: u32,
    bit_depth: u8,
) -> BenchmarkResult {
    let raw_bytes = (samples.len() * samples[0].len() * samples[0][0].len() * (bit_depth as usize / 8)) as u64;
    
    // Measure Encoding
    let t_enc_start = Instant::now();
    let compressed = loom_core::container::session::encode_session(
        samples, &vec!["t".to_string(); samples.len()], sample_rate, bit_depth, 512, None, None, None
    ).expect("Encode failed");
    let encode_time = t_enc_start.elapsed().as_secs_f64();
    
    // Measure Decoding
    let t_dec_start = Instant::now();
    let _decoded = loom_core::container::session::decode_session(&compressed)
        .expect("Decode failed");
    let decode_time = t_dec_start.elapsed().as_secs_f64();
    
    let comp_bytes = compressed.len() as u64;
    let ratio = raw_bytes as f64 / comp_bytes as f64;
    
    BenchmarkResult {
        dataset_name: "Session_Benchmark".to_string(),
        raw_bytes,
        compressed_bytes: comp_bytes,
        compression_ratio: ratio,
        encode_time_secs: encode_time,
        decode_time_secs: decode_time,
        encode_mb_per_sec: (raw_bytes as f64 / 1_000_000.0) / encode_time,
        decode_mb_per_sec: (raw_bytes as f64 / 1_000_000.0) / decode_time,
    }
}
```

---

## 8. References

1. **European Broadcasting Union (EBU) (1988):** *SQAM - Sound Quality Assessment Material User's Handbook.* EBU Document Tech 3253.
2. **Criterion.rs:** *Statistics-driven microbenchmarking framework for Rust.* [https://bheisler.github.io/criterion.rs/book/](https://bheisler.github.io/criterion.rs/book/)
3. **Hydrogenaudio:** *Lossless Audio Comparison Study.* [https://wiki.hydrogenaud.io/index.php?title=Lossless_comparison](https://wiki.hydrogenaud.io/index.php?title=Lossless_comparison)

---

## 9. Open Research Questions & Future Work

1. **Automated CI Regression Tracking:** Integrating automated benchmark runs into GitHub Actions CI to fail pull requests that regress decoding speed by $> 3\%$ or compression ratio by $> 0.1\%$.
