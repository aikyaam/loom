# Loom Developer Log (DevLog)

**Project:** Loom Codec & Multitrack Session Container  
**Language:** Rust 1.80+  
**Architecture:** Hybrid FLAC Container (`.loom`)  
**Test Suite:** 28 passing unit and integration tests (`cargo test`)

---

## 1. Problem Statement (Why We Built Loom)

Traditional lossless audio formats (FLAC, WAV, ALAC) were designed for linear, single-track playback. In modern Digital Audio Workstation (DAW) music production, this introduces severe inefficiencies:

* **Multitrack Redundancy:** Compressing 20 stems individually duplicates shared timing, transients, and microphone leakage across files.
* **Destructive Edits:** Applying a simple fade-out or muting a region requires rendering and writing a brand-new audio file to disk.
* **Storage Bloat:** Saving successive mix revisions (`Mix_v1`, `Mix_v2`) duplicates identical, unmodified audio stems.

---

## 2. What We Built

We designed and implemented **Loom**, a research-driven lossless audio codec and session container ecosystem:

### Core Codec Engine (`loom-core`)
* **Predictor Mechanics:** Polynomial fixed predictors (Orders 0-4), Levinson-Durbin LPC, Burg Maximum Entropy Method (MEM) lattice filters ($|k_i| < 1$), and Normalized LMS (NLMS) sample-adaptive gradient filters.
* **Reversible Transforms:** 5/3 CDF Integer Wavelet Lifting and Integer MDCT subband decomposition.
* **Entropy State Machines:** Partitioned Golomb-Rice coding, Range Asymmetric Numeral Systems (rANS), and Table-Based ANS (tANS) finite state machines.
* **Multitrack Decorrelation:** Mid-Side stereo matrixing, Inter-stem residual prediction coupling, Karhunen-Loève Transform (KLT) covariance matrices, and Directed Acyclic Graph (DAG) cross-channel topology solvers.
* **Non-Destructive Edits:** On-the-fly rendering of linear, exponential, sigmoidal, and cosine fade curves, gain automation envelopes, and mute regions during decoding.
* **Frame Version Diffing:** Frame-level MD5 fingerprinting and delta compression (`encode_diff`, `apply_diff`).

### CLI Tooling & Diagnostics (`loom-cli`)
* Implemented 17 subcommands: `encode`, `decode`, `verify`, `encode-session`, `decode-session`, `extract`, `edit`, `render`, `diff`, `apply-diff`, `play`, `tag`, `benchmark`, `analyze`, `compare`, `inspect`, `info`.

### Benchmarking Suite (`loom-bench`)
* Created Criterion micro-benchmarks and a full empirical benchmark runner (`run_full_benchmark`).

### Academic Research Platform (`research/`)
* Authored 24 publication-grade academic research manuscripts (`01-flac.md` to `24-container-format-taxonomy.md`) covering DSP math, information theory, and multitrack container design.

---

## 3. Key Technical Challenges & Fixes

| Issue / Bug | Root Cause | Engineering Solution |
| :--- | :--- | :--- |
| **LaTeX `'_' allowed only in math mode`** | Raw underscores inside `\text{...}` blocks exited math mode in KaTeX/MathJax renderers. | Replaced underscores inside `\text{...}` with hyphens (e.g. `\text{track-meta-m}`, `\text{mute-start}`). |
| **Broken GitHub Math Rendering** | GitHub Markdown requires empty lines surrounding `$$...$$` display math blocks. | Automated spacing script to insert blank lines before and after every display math block across all 24 papers. |
| **Broken Table of Contents Anchors** | Including inline LaTeX math (`$\mathcal{O}(\log K)$`) inside Markdown heading titles broke anchor links. | Cleaned heading titles to plain text format (e.g. `(O(log K))`) across all research papers. |
| **Mid-Side Reconstruction Rounding** | Signed integer arithmetic right shift (`>> 1`) introduced off-by-one errors on negative odd integers. | Implemented bit-exact formulas: `left = mid + ((side + 1) >> 1)` and `right = mid - (side >> 1)`. |
| **SIMD Hardware Vectorization** | Standard serial autocorrelation loops constrained predictor search throughput. | Implemented 256-bit AVX2/SSE2 vector routines and AArch64 ARM NEON (`autocorr_neon`) primitives using vector intrinsics. |

---

## 4. Empirical Performance Highlights

* **Multitrack Session Savings:** 8-stem DAW session compresses to **488,503 bytes** (13.85% ratio) in Loom vs. **647,112 bytes** (18.34% ratio) for 8 separate FLAC files: **24.51% higher storage savings**.
* **Single-Track Tonal Compression:** Loom Level 5 achieves **16.65% compression ratio** on sine sweeps vs FLAC default **18.34%**.
* **Decoding Throughput:** **176.83 MB/s** for drum transient signals; **141.29 MB/s** for silence backfill.
* **Random Access Seek Latency:** **4.42 ms** range extraction latency for a 1-second slice.
