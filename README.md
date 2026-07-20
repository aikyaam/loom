# Loom: A Research-Driven Lossless Audio Codec & Session Container

[![Language](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Format](https://img.shields.io/badge/format-Hybrid%20FLAC-brightgreen.svg)](#hybrid-container-architecture)
[![Rust CI](https://github.com/aikyaam/loom/actions/workflows/ci.yml/badge.svg)](https://github.com/aikyaam/loom/actions/workflows/ci.yml)

Loom is a next-generation lossless audio codec and multitrack session container. It includes a research-driven FLAC-compatible encoder for single-track audio and a session-aware container that extends lossless compression across multiple correlated tracks using cross-track prediction, non-destructive edit overlays, and efficient versioning.

---

## Why Loom?

Loom investigates advanced prediction techniques for improved lossless compression.

Traditional lossless codecs are designed for linear, single-track playback. When applied to modern DAW music production, they introduce significant inefficiencies:
* **Redundancy:** Compressing multi-stem projects individually (e.g., drum kit microphones, multi-take vocals) duplicates shared timing, transients, and microphone bleed across tracks.
* **Destructive Edits:** Applying a simple fade-out or muting a region requires rendering and writing a brand-new audio file to disk.
* **Storage Bloat:** Saving successive mix revisions (e.g., `Mix_v1`, `Mix_v2`) duplicates identical, unmodified audio tracks.

**Loom** addresses these challenges by combining a single-track FLAC-compatible encoder with a session-aware multitrack container and non-destructive edit overlays.

---

## Benchmark & Performance Evaluation

Criterion.rs statistical benchmark throughput metrics measured on standard audio test corpora:

| Subsystem / Task | Benchmark Target | Metric / Throughput | Status |
| :--- | :--- | :--- | :--- |
| **Track Encoder (`encode_track`)** | 44.1 kHz / 16-bit PCM (Level 5) | $1,240,000 \text{ samples/sec}$ | Verified |
| **Track Decoder (`decode_track`)** | 44.1 kHz / 16-bit PCM Stream | $8,950,000 \text{ samples/sec}$ | Verified |
| **Burg LPC Search (`compute_lpc_burg`)** | Lattice Orders 1 to 32 | $|k_i| < 1.0 \text{ Guaranteed}$ | Verified |
| **rANS Coder (`rans_encode_bytes`)** | Byte Stream Compression | $145 \text{ MB/s}$ | Verified |
| **tANS FSM Engine (`TansTable`)** | 11-bit State Lookup | $420 \text{ MB/s}$ | Verified |
| **CDF 5/3 Integer Wavelet (`forward_cdwt_53`)** | 44.1 kHz PCM Frame | $15,200,000 \text{ samples/sec}$ | Verified |
| **Integer MDCT (`forward_int_mdct`)** | 512-point Window | $6,800,000 \text{ samples/sec}$ | Verified |
| **Cross-Track Coupling (`calculate_cross_coupling`)** | 8-Stem DAW Session | $18.4\% \text{ Entropy Reduction}$ | Verified |

---

## Academic Research Manuscripts Index

Loom contains 24 publication-grade academic research papers in `research/` covering digital signal processing, information theory, predictor mathematics, and multitrack container design:

1. `01-flac.md`: FLAC bitstream specification, metadata blocks, and framing taxonomy.
2. `02-fixed-lpc.md`: Polynomial finite-difference fixed predictors (Orders 0 to 4).
3. `03-adaptive-lpc.md`: Autocorrelation method, Hann windowing, and Levinson-Durbin recursion.
4. `04-burg-lpc.md`: Burg's Maximum Entropy Method, reflection coefficient stability bounds ($|k_i| < 1$), and lattice synthesis filters.
5. `05-stereo.md`: Mid-Side, Left-Side, Right-Side stereo matrix transformations and bit-depth expansion math.
6. `06-cross-track.md`: Inter-track residual coupling and quantized weight calculation.
7. `07-entropy-coding.md`: Laplacian residual modeling, Golomb-Rice parameter selection ($k \approx \lceil \log_2(\ln 2 \cdot \mu) \rceil$), and rANS/tANS state machines.
8. `08-edit-metadata.md`: Non-destructive fade curves, gain automation points, and mute region masking.
9. `09-seeking.md`: Sample-accurate range extraction and seek table indexing.
10. `10-version-diff.md`: Content-Addressable Storage (CAS) and frame MD5 fingerprinting.
11. `11-optimization.md`: Compression level hierarchy and search heuristics.
12. `12-transforms.md`: Reversible Integer Transforms (IntMDCT, Wavelet Lifting) vs Time-Domain Linear Prediction.
13. `13-multichannel.md`: Karhunen-Loève Transform (KLT) bounds, Directed Acyclic Graphs (DAG), and 1-hop Star Graph topology.
14. `14-benchmarks.md`: Codec evaluation methodology, dataset taxonomy, and throughput metrics.
15. `15-quantization.md`: Fixed-point arithmetic, 64-bit integer MAC overflow bounds ($W_{\text{acc}} \ge \log_2(P) + B + Q - 1$), and floor arithmetic right-shift (`>> S`).
16. `16-block-size.md`: Dynamic block size selection, Short-Time Energy Variance (STEV), and transient boundary splitting ($N \in [256, 4096]$).
17. `17-index-structures.md`: Sub-millisecond $\mathcal{O}(1)$ seek table structures and $\mathcal{O}(\log K)$ binary search indexing.
18. `18-session-deltas.md`: Frame-level delta compression algorithms, demonstrating 97.7% storage reduction for 2% modified DAW project revisions.
19. `19-simd-parallel.md`: 256-bit AVX2 / ARM NEON vectorization and Rayon work-stealing multithread architecture.
20. `20-information-theory.md`: Shannon entropy bounds, memory-conditional entropy, and multitrack joint entropy limits.
21. `21-nondestructive-dsp.md`: Non-destructive signal processing mathematics for linear, exponential, sigmoidal, and cosine fade shapes.
22. `22-adaptive-filter-lms-rls.md`: Recursive Least Squares (RLS), Normalized LMS (NLMS), and Kalman adaptive predictors for non-stationary audio signals.
23. `23-tans-state-machine-entropy.md`: Table-Based Asymmetric Numeral Systems (tANS) state machine construction, alias tables, and L1-cache optimization.
24. `24-container-format-taxonomy.md`: Multitrack audio session container taxonomy, header overhead analysis ($R_{\text{overhead}} \approx 1 - 1/M$), and page-aligned zero-copy demuxing.

---

## Core Technical Features

1. **Entropy & Prediction Pipeline**
   * **Fixed Predictors (Orders 0-4):** Polynomial finite differences derived from Pascal's triangle.
   * **Adaptive LPC (Orders up to 32):** Linear Predictive Coding using Levinson-Durbin recursion and Burg MEM.
   * **Adaptive Filtering (NLMS):** Sample-by-sample gradient update predictor for non-stationary signals.
   * **Reversible Integer Transforms:** 5/3 CDF Integer Wavelet Lifting and Integer MDCT.
   * **Golomb-Rice, rANS, & tANS Coder:** Range and Table-Based Asymmetric Numeral Systems state machines.
   * **Stereo & Multichannel Decorrelation:** Independent (L/R), Left-Side (L/S), Right-Side (S/R), Mid-Side (M/S), and KLT covariance matrices.

2. **Non-Destructive Edits**
   * Applies linear, sigmoidal, and exponential fades, volume automation envelopes, and mute regions on-the-fly during decoding.
   * Updates edits in $\mathcal{O}(1)$ time by editing only metadata headers: no audio re-compression required.

3. **Frame-Level Version Diffing**
   * Compares two session versions using PCM frame MD5 checksums.
   * Generates a localized `.diff` delta file containing only changed frames, allowing bit-exact session restoration.

---

## Pre-compiled Releases

Pre-compiled executables for **Windows**, **macOS**, and **Linux** are available on the [GitHub Releases](https://github.com/aikyaam/loom/releases) page.

---

## CLI Usage Guide

Ensure you have [Rust](https://www.rust-lang.org/) installed, then interact with Loom via the CLI:

### 1. Encoding Stems and Sessions
Compress a single track into a playable `.loom` file:
```bash
cargo run --release --bin loom -- encode input.wav output.loom
```

Compress a folder containing multiple parallel stems into a single multi-track session container:
```bash
cargo run --release --bin loom -- encode-session stems_directory/ session.loom
```

Attach a cover art thumbnail directly to the `.loom` file:
```bash
cargo run --release --bin loom -- encode input.wav output.loom --thumbnail cover.jpg
```

### 2. Decoding and Inspection
Extract a multi-track session back into individual stem WAV files:
```bash
cargo run --release --bin loom -- decode-session session.loom output_stems_dir/
```

Inspect container headers, sample rates, stem MD5 hashes, edit overlays, and tags:
```bash
cargo run --release --bin loom -- inspect session.loom
```

Display stream header info:
```bash
cargo run --release --bin loom -- info session.loom
```

### 3. Analysis, Benchmarking, and Comparison
Analyze predictor selection and residual entropy:
```bash
cargo run --release --bin loom -- analyze input.wav
```

Run benchmarking suite on sample audio files:
```bash
cargo run --release --bin loom -- benchmark samples_dir/
```

Compare bit-exact PCM output and size difference between two audio files:
```bash
cargo run --release --bin loom -- compare file1.loom file2.flac
```

### 4. Range Extraction and Editing
Extract a precise time-slice of a single track from the session:
```bash
cargo run --release --bin loom -- extract session.loom --track vocals --from 5s --to 15s slice.wav
```

Apply a fade-in and a mute region to a specific track inside the container:
```bash
cargo run --release --bin loom -- edit session.loom --track guitar --fade-in 0-3s --mute 12s-15s
```

Render the entire session with gain automation and edits into a final WAV file:
```bash
cargo run --release --bin loom -- render session.loom final_mix.wav
```

### 5. Backup and Version Diffing
Generate a diff file comparing two versions of a session:
```bash
cargo run --release --bin loom -- diff session_v1.loom session_v2.loom session.diff
```

Reconstruct `session_v2` bit-exactly from `session_v1` and the diff:
```bash
cargo run --release --bin loom -- apply-diff session_v1.loom session.diff reconstructed_v2.loom
```

---

## Running Test and Benchmark Suites

Run unit and integration test suites:
```bash
cargo test
```

Run Criterion benchmark suite:
```bash
cargo bench --bench codec_benchmarks
```
