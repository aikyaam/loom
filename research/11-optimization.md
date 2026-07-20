# 09-optimization-strategies.md

## Compression Optimization Research for Loom

This document summarizes optimization strategies from FLAC reference encoder, academic research, and community findings applicable to Loom's lossless audio codec.

---

## 1. Compression Level Hierarchy (FLAC Model)

FLAC's 9 compression levels (0-8) progressively apply more sophisticated techniques:

| Level | Mid-Side | Max LPC Order | Partition Order | Apodization | Escape | Exhaustive | QLP Precision Search |
|-------|----------|---------------|-----------------|-------------|--------|------------|---------------------|
| 0     | Off      | 0 (fixed)     | 0-3             | tukey(0.5)  | No     | No         | No                  |
| 1     | Loose    | 0 (fixed)     | 0-3             | tukey(0.5)  | No     | No         | No                  |
| 2     | On       | 0 (fixed)     | 0-3             | tukey(0.5)  | No     | No         | No                  |
| 3     | Off      | 6             | 0-4             | tukey(0.5)  | No     | No         | No                  |
| 4     | Loose    | 8             | 0-4             | tukey(0.5)  | No     | No         | No                  |
| 5     | On       | 8             | 0-5             | tukey(0.5)  | No     | No         | No                  |
| 6     | On       | 8             | 0-6             | subdivide_tukey(2) | No | No | No |
| 7     | On       | 12            | 0-6             | subdivide_tukey(2) | No | No | No |
| 8     | On       | 12            | 0-6             | subdivide_tukey(3) | Yes | Yes | Yes |

**Loom adoption**: Implement a similar `--compression-level 0..8` CLI flag. Default to level 5 (balanced).

---

## 2. Apodization Windows

Apodization (windowing) reduces spectral leakage before LPC analysis, improving predictor accuracy.

### Standard Windows
- **tukey(α)**: Cosine-tapered rectangular window. α=0.5 is FLAC default.
- **subdivide_tukey(N)**: Splits block into N sub-blocks, applies tukey to each. Better for transient detection.
- **punchout_tukey(N)**: Used by CUEtools.Flake. Excludes transients from LPC analysis.

### Implementation Notes
```rust
enum Apodization {
    Tukey(f64),              // α ∈ [0, 1]
    SubdivideTukey(u32),     // N sub-blocks
    PunchoutTukey(u32),      // N punchout regions
    PartialTukey(u32),       // FLAC 1.4.3+
    Welch,
    Hann,
}
```

**Recommendation**: Support `tukey(0.5)`, `subdivide_tukey(2)`, `subdivide_tukey(3)`, `punchout_tukey(3)`. Allow multiple windows per frame; encoder picks best.

---

## 3. LPC Coefficient Optimization

### 3.1 Levinson-Durbin (Current)
Standard O(p²) algorithm for solving Yule-Walker equations from autocorrelation. Used by FLAC, Shorten, Loom.

### 3.2 Double-Precision Autocorrelation (CUEtools.Flake)
FLAC uses `float` (32-bit) for autocorrelation; CUEtools.Flake uses `double` (64-bit).
- **Gain**: ~0.2–1.2% better compression on piano/transient-heavy material
- **Cost**: ~2x slower autocorrelation
- **Loom**: Use `f64` for autocorrelation internally; quantize coefficients to `i32` for storage.

### 3.3 IRLS (Iteratively Reweighted Least Squares)
Minimizes L1 norm (absolute deviation) instead of L2 (least squares). Matches Rice coding cost better.
- **Gain**: Up to 0.2% extra on electronic/synthetic material
- **Cost**: 20–60x slower than Levinson-Durbin
- **Hybrid approach**: Use Levinson-Durbin result as initial guess for 2–3 IRLS iterations (IRLS-post).

**Loom**: Implement as optional `--irls-iterations N` flag at high compression levels.

### 3.4 QLP Coefficient Precision Search
FLAC searches precision 1–16 bits for quantized LPC coefficients.
- Higher precision = better prediction but more header bits
- Optimal depends on block size and bit depth

**Loom**: Search precision 4–16 bits at compression level ≥ 5.

---

## 4. Stereo Decorrelation

### Mid-Side (MS) Coding
```
mid = (left + right) >> 1
side = left - right
```
- **Gain**: 10–30% on correlated stereo
- **Loose MS**: Only use MS when it saves bits (per-frame decision)
- **Loom**: Implement per-frame loose MS (default at level ≥ 2).

### Cross-Track (Phase 2 Feature)
Loom's cross-track prediction generalizes MS to N tracks. Use same per-frame selection logic.

---

## 5. Residual Coding Optimizations

### 5.1 Partitioned Rice Coding (Current)
- Partition order 0–8 (2⁰ to 2⁸ partitions)
- 4-bit Rice parameter (0–15), escape code 15 = raw
- Per-partition optimal k search

### 5.2 Rice Parameter Search Strategies
| Strategy | Description | Speed | Quality |
|----------|-------------|-------|---------|
| Estimate | k ≈ log2(mean(|residual|)) | Fast | Good |
| Exhaustive | Try all k=0..15 per partition | Slow | Optimal |
| Limited | Search ±N around estimate | Medium | Near-optimal |

**Loom**: Default to estimate ±2 at level ≤ 4; exhaustive at level ≥ 6.

### 5.3 Escape Coding
When Rice coding expands data (e.g., noisy/verbatim-like residuals), use raw binary encoding.
- FLAC escape: 5-bit bps + raw samples
- **Gain**: Prevents expansion on difficult frames

### 5.4 Alternative Entropy Coders (Research)
| Coder | vs Rice | Complexity |
|-------|---------|------------|
| Exponential Golomb | ~1–3% better on heavy tails | Low |
| Huffman (adaptive) | ~2–4% better | Medium |
| ANS / Range / Arithmetic | ~3–5% better | High |
| Context-adaptive | Up to 5% on structured residuals | Very High |

**Loom**: Reserve residual coding method IDs in container format for future ANS/arithmetic coding. Keep Rice as default for decode speed.

---

## 6. Block Size Optimization

### Variable Block Size
- Transients → smaller blocks (better prediction)
- Stationary → larger blocks (less header overhead)
- FLAC supports but reference encoder uses fixed 4096/4608

**Loom**: Implement variable block size at level ≥ 6:
- Analyze signal energy derivative
- Split at transient boundaries
- Max 4096, min 192 (or 1152 for ≤ 48kHz subset)

### Optimal Block Size Search
Dynamic programming: find partition points minimizing total bits.

---

## 7. CPU Optimizations

### SIMD Acceleration (Priority: High)
| Operation | SSE2 | SSSE3 | SSE4.1 | AVX2 | NEON |
|-----------|------|-------|--------|------|------|
| Autocorrelation | ✓ | | ✓ | ✓ | ✓ |
| LPC residual | | ✓ | ✓ | ✓ | ✓ |
| Rice coding | | | | ✓ | |
| CRC | | | | | |

**Loom**: Use `std::arch` / `packed_simd` / `portable_simd` for x86_64 + ARM NEON. Target: autocorrelation + residual computation first.

### Multi-threading (Priority: Medium)
- Frame-level parallelism (independent frames)
- Channel-level parallelism (independent subframes)
- Thread pool with work stealing

---

## 8. Encoder Search Space Pruning

At high compression levels, exhaustive search is slow. Pruning strategies:

1. **Early abandonment**: Stop evaluating LPC order p if residual bits > best_fixed_bits
2. **Order bounding**: Skip p if p-1 residual bits didn't improve by > threshold
3. **Precision bounding**: Don't search precision > needed for given block size
4. **Window pre-selection**: Fast energy analysis to pick 2–3 candidate windows

---

## 9. Container-Level Optimizations (Loom-Specific)

### Session-Level
- Cross-track predictor reuse (store coefficients once, reference from multiple tracks)
- Shared seek index across tracks
- Edit metadata: non-destructive fades/gain avoid re-encoding

### Frame-Level
- Variable block size per track (not global)
- Per-frame predictor type selection (Constant/Verbatim/Fixed/LPC)
- Per-channel subframe type selection

---

## 10. Proposed Loom Compression Levels

| Level | Name | Techniques Enabled | Target Use Case |
|-------|------|-------------------|-----------------|
| 0 | fast | Fixed only, tukey(0.5), no MS | Real-time encoding |
| 1 | fast+ | + loose MS | Fast, decent compression |
| 2 | balanced | + LPC order 6, subdivide_tukey(2) | Default |
| 3 | balanced+ | + order 8, partition order 5 | Good compression |
| 4 | high | + order 12, exhaustive k-search, escape | Archival |
| 5 | high+ | + subdivide_tukey(3), QLP precision search | Max compression |
| 6 | insane | + double-prec autocorr, IRLS-post(2) | Offline, best ratio |
| 7 | insane+ | + punchout_tukey(3), variable block size | Extreme |
| 8 | maximum | All above + exhaustive model search | Research/precision search |

---

## 11. Benchmarking Methodology

**Corpus**: 
- CDDA: 44.1kHz 16-bit stereo (various genres)
- Hi-res: 96kHz 24-bit stereo
- Multichannel: 48kHz 24-bit 5.1
- Synthetic: silence, sine sweeps, noise, chiptune

**Metrics**:
- Compression ratio vs FLAC -5, -8
- Encode time (single-threaded, multi-threaded)
- Decode time (must stay fast)
- CPU/memory usage

**Target**: 
- Level 5: Match FLAC -8 ratio, ≤ 2x FLAC -8 encode time
- Level 8: Beat FLAC -8 by ≥ 1%, decode within 1.5x FLAC

---

## References

1. RFC 9639: FLAC Specification
2. FLAC source: `src/libFLAC/stream_encoder.c` (compression levels, apodization, search)
3. CUEtools.Flake: Double-precision autocorrelation, punchout_tukey
4. Hydrogenaudio thread "New FLAC compression improvement": IRLS, benchmarking
5. "Optimized FPGA Implementation of Audio Compression Using LPC and Golomb Coding" (2025): Exponential Golomb alternative
6. encode.su thread "Encoding FLAC residuals (alternatives to Rice encoding)": ANS/arithmetic coding discussion
7. FLAC-dev mailing list: Double vs single precision autocorrelation debate

---