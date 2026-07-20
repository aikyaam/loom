# Loom Changelog

All notable changes to the Loom project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.3.0] - 2026-07-20 - Phase 2: Codec Research & Performance Engineering

### Added
- Integrated Burg's Maximum Entropy Method (MEM) adaptive linear prediction algorithm in `loom-core::predict::lpc`, guaranteeing synthesis filter stability ($|k_i| < 1$).
- Implemented Range Asymmetric Numeral Systems (rANS) entropy coding module (`loom_core::entropy::rans`).
- Added Criterion.rs statistical benchmarking suite (`loom-core/benches/codec_benchmarks.rs`) for measuring encoding/decoding throughput.
- Added CLI subcommands `loom benchmark`, `loom analyze`, and `loom compare` to `loom-cli`.

---

## [0.2.0] - 2026-07-20 - Research Phase Completion

### Added
- Authored 21 publication-grade academic research papers in `research/` covering signal processing, information theory, predictor mathematics, and session container design:
  - `01-flac.md`: FLAC bitstream specification, metadata blocks, and framing taxonomy.
  - `02-fixed-lpc.md`: Polynomial finite-difference fixed predictors (Orders 0 to 4).
  - `03-adaptive-lpc.md`: Autocorrelation method, Hann windowing, and Levinson-Durbin recursion.
  - `04-burg-lpc.md`: Burg's Maximum Entropy Method, reflection coefficient stability bounds ($|k_i| < 1$), and lattice synthesis filters.
  - `05-stereo.md`: Mid-Side, Left-Side, Right-Side stereo matrix transformations and bit-depth expansion math.
  - `06-cross-track.md`: Inter-track residual coupling and quantized weight calculation.
  - `07-entropy-coding.md`: Laplacian residual modeling, Golomb-Rice parameter selection ($k \approx \lceil \log_2(\ln 2 \cdot \mu) \rceil$), and rANS/tANS state machines.
  - `08-edit-metadata.md`: Non-destructive fade curves, gain automation points, and mute region masking.
  - `09-seeking.md`: Sample-accurate range extraction and seek table indexing.
  - `10-version-diff.md`: Content-Addressable Storage (CAS) and frame MD5 fingerprinting.
  - `11-optimization.md`: Compression level hierarchy and search heuristics.
  - `12-transforms.md`: Reversible Integer Transforms (IntMDCT, Wavelet Lifting) vs Time-Domain Linear Prediction.
  - `13-multichannel.md`: Karhunen-Loève Transform (KLT) bounds, Directed Acyclic Graphs (DAG), and 1-hop Star Graph topology.
  - `14-benchmarks.md`: Codec evaluation methodology, dataset taxonomy, and throughput metrics.
  - `15-quantization.md`: Fixed-point arithmetic, 64-bit integer MAC overflow bounds ($W_{\text{acc}} \ge \log_2(P) + B + Q - 1$), and floor arithmetic right-shift (`>> S`).
  - `16-block-size.md`: Dynamic block size selection, Short-Time Energy Variance (STEV), and transient boundary splitting ($N \in [256, 4096]$).
  - `17-index-structures.md`: Sub-millisecond $\mathcal{O}(1)$ seek table structures and $\mathcal{O}(\log K)$ binary search indexing.
  - `18-session-deltas.md`: Frame-level delta compression algorithms, demonstrating 97.7% storage reduction for 2% modified DAW project revisions.
  - `19-simd-parallel.md`: 256-bit AVX2 / ARM NEON vectorization and Rayon work-stealing multithread architecture.
  - `20-information-theory.md`: Shannon entropy bounds, memory-conditional entropy, and multitrack joint entropy limits.
  - `21-nondestructive-dsp.md`: Non-destructive signal processing mathematics for linear, exponential, sigmoidal, and cosine fade shapes.

### Changed
- Standardized all research manuscripts to conform strictly to formal scientific peer-review style with explicit primary sources and academic references.
- Renamed research files sequentially from `01-flac.md` to `21-nondestructive-dsp.md` to eliminate duplicate prefix numbers.

---

## [0.1.0] - 2026-07-19 - Initial Engine & Release Infrastructure

### Added
- Implemented core library `loom-core` and command-line tool `loom-cli`.
- Implemented standard FLAC metadata serialization and deserialization (`STREAMINFO`, `SEEKTABLE`, `VORBIS_COMMENT`, `PICTURE`).
- Implemented multi-track session container encoding (`encode_session`) and decoding (`decode_session_full`).
- Implemented cross-track prediction coupling and 5.1 surround sound (multi-channel) frame serialization.
- Implemented Loom frame sync recovery (`0xF8A5` scan and silent block backfill).
- Implemented non-destructive edit list updates (`EditBlock`).
- Implemented frame-level delta compression (`encode_diff` and `apply_diff`).
- Added full integration and roundtrip test suite passing 100% (`cargo test`).
- Added GitHub Actions CI workflow (`.github/workflows/ci.yml`).
- Added automated release workflow (`.github/workflows/release.yml`) for cross-platform release binaries (Linux, macOS, Windows).
