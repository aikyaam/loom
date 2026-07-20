# Research Paper 13: Multichannel Decorrelation Theory: Karhunen-Loève Transform (KLT), Directed Dependency Graphs, and Cross-Track Residual Coupling

**Author:** Loom Codec Research Group  
**Status:** Complete  
**Date:** July 2026  
**Sources:** Openshaw (2002), Liebchen (2006), Gersho & Gray (1992)

---

## 1. Problem Statement

Modern audio recording sessions (DAW multi-track projects) consist of tens or hundreds of correlated stem tracks (such as multi-microphone drum setups with kick, snare, hi-hat, overheads, and room mics; layered guitar takes; multi-mic classical ensembles; and backing vocal harmonies).

Traditional audio codecs compress tracks independently or limit interchannel decorrelation to 2-channel stereo pairs (Mid/Side, Left/Side, Right/Side). Compressing 32 parallel stems individually results in severe spatial redundancy:
1. **Acoustic Leakage:** Microphone bleed across drum and vocal tracks creates near-identical phase and frequency components across multiple files.
2. **Shared Room Acoustics:** Reverb decay and ambient noise floors are identical across stems.
3. **Session Timing & Transients:** Instrument attacks occur simultaneously across tracks.

The core challenge of **Loom's Cross-Track Engine** is developing an $M$-channel decorrelation framework that maximizes total entropy reduction across $M$ tracks while enforcing **Strict $O(1)$ Independent Track Extraction**, ensuring that a user reading a single stem from a 64-track `.loom` session container does not need to decode the remaining 63 tracks.

---

## 2. Historical Background

- **Stereo Mid-Side Decorrelation (1930s, Blumlein):** Alan Blumlein introduced sum-and-difference matrixing ($M = \frac{L+R}{2}, S = L - R$) for stereophonic disc cutting.
- **WavPack Multichannel Decorrelation (2002, Openshaw):** WavPack introduced pairwise channel decorrelation, allowing any channel to be subtracted from an adjacent channel with an adaptive weight factor.
- **MPEG-4 ALS Multichannel Joint Coding (2006):** MPEG-4 ALS introduced Frequency-Domain Inter-Channel Prediction and adaptive Channel Order selection. However, decoding any single channel required decoding preceding dependent channels in a sequential chain, causing high seeking latency.
- **Loom Directed Acyclic Dependency Graph (2026):** Loom formulates multitrack decorrelation as finding a **Minimum Spanning Tree (MST)** over a complete correlation graph, restricting dependency depth to $\le 1$ to guarantee fast $O(1)$ single-track extraction.

---

## 3. Mathematical Derivation

### 3.1 Multichannel Covariance Matrix & Karhunen-Loève Transform (KLT)

Let $\mathbf{x}[n] = [x_1[n], x_2[n], \dots, x_M[n]]^T$ be an $M$-dimensional vector of audio samples across $M$ tracks at sample index $n$.

The spatial covariance matrix $\mathbf{\Sigma}_{xx} \in \mathbb{R}^{M \times M}$ is defined as:

$$\mathbf{\Sigma}_{xx} = E\{ \mathbf{x}[n] \mathbf{x}^T[n] \} \approx \frac{1}{N} \sum_{n=0}^{N-1} \mathbf{x}[n] \mathbf{x}^T[n]$$

The **Karhunen-Loève Transform (KLT)** diagonalizes $\mathbf{\Sigma}_{xx}$ via its eigenvector decomposition:

$$\mathbf{\Sigma}_{xx} = \mathbf{V} \mathbf{\Lambda} \mathbf{V}^T$$

where $\mathbf{V}$ is an orthogonal matrix of eigenvectors, and $\mathbf{\Lambda} = \text{diag}(\lambda_1, \lambda_2, \dots, \lambda_M)$ contains eigenvalues representing variance along principal axes.

While KLT theoretically achieves optimal energy compaction, it suffers from two major flaws for lossless session compression:
1. $\mathbf{V}$ requires floating-point matrix multiplication, which is non-reversible over integers $\mathbb{Z}^M \to \mathbb{Z}^M$.
2. Every output track $y_i[n]$ depends on **all $M$ input tracks**, destroying single-track extraction capabilities.

---

### 3.2 Directed Acyclic Graph (DAG) Cross-Track Prediction

Instead of full KLT matrix inversion, Loom models inter-track dependency as a **Directed Acyclic Graph (DAG)** operating on residual signals after per-track LPC prediction.

Let $E_t[n]$ be the LPC prediction residual of track $t$ ($t \in \{0, 1, \dots, M-1\}$).  
Let Track 0 be designated as the **Primary Reference Track** (e.g., standard stereo mix or main drum track).

For target track $t > 0$, we model its residual $E_t[n]$ as a linear function of reference residual $E_0[n]$:

$$\hat{E}_t[n] = \frac{W_t}{256} \cdot E_0[n]$$

where $W_t \in [-128, 127]$ is an 8-bit signed quantized coupling weight.

The cross-track residual $e_t[n]$ is computed as:

$$e_t[n] = E_t[n] - \left( \frac{W_t \cdot E_0[n]}{256} \right)$$

### 3.3 Optimal Coupling Weight Calculation

To minimize the residual energy $\sum_n e_t^2[n]$, we take the derivative with respect to $W_t$:

$$\frac{\partial}{\partial W_t} \sum_{n=0}^{N-1} \left( E_t[n] - \frac{W_t}{256} E_0[n] \right)^2 = 0$$

Solving for $W_t$:

$$W_t = \text{round}\left( 256 \cdot \frac{\sum_{n=0}^{N-1} E_t[n] E_0[n]}{\sum_{n=0}^{N-1} E_0^2[n]} \right)$$

### 3.4 Bit-Cost Thresholding & Graph Optimization

Cross-track prediction is applied **only if** the total bit savings exceed the metadata overhead of storing weight $W_t$ and reference track index (8 bits total):

$$\Delta \text{Bits} = \text{RiceBits}(E_t) - \left( \text{RiceBits}(e_t) + 8 \right)$$

If $\Delta \text{Bits} > 8 \text{ bits}$, cross-track prediction is enabled for track $t$ in that frame. Otherwise, track $t$ is encoded independently.

---

## 4. Algorithm Explanation

```
           +---------------------------------------------------+
           | Input Multitrack Frame: M Tracks, N Samples/Block |
           +---------------------------------------------------+
                                     |
                                     v
             Step 1: Compute Per-Track LPC Residuals E_0..E_{M-1}
                                     |
                                     v
             Step 2: Designate Reference Track E_0 (Track 0)
                                     |
              +----------------------+----------------------+
              |                                             |
              v                                             v
     Track 0 (Primary)                            Track t (t = 1..M-1)
              |                                             |
   Encode E_0 Directly                     1. Compute W_t = round(256 * Cov(E_t, E_0) / Var(E_0))
              |                            2. Calculate e_t = E_t - (W_t * E_0 >> 8)
              |                            3. Evaluate Bit-Cost Delta
              |                                             |
              |                   +-------------------------+-------------------------+
              |                   |                                                   |
              |                   v                                                   v
              |         If Delta Bits > 8:                                   Else:
              |         Store W_t, Set Ref=0,                                Store Track t
              |         Encode e_t (Cross-Residual)                          Independently
              |                   |                                                   |
              +-------------------+---------------------------------------------------+
                                  |
                                  v
                      Serialize Container Frame
```

---

## 5. Complexity Analysis

Let $M$ be the number of tracks (e.g., $M = 32$) and $N$ be the frame block size (e.g., $N = 4096$).

| Phase | Operations per Frame | Memory Complexity | Parallelizability |
| :--- | :--- | :--- | :--- |
| **Per-Track LPC** | $M \cdot (N \cdot P + P^2)$ ops | $O(M \cdot N)$ | 100% Parallel across tracks (Rayon) |
| **Cross-Track Covariance** | $(M - 1) \cdot N$ multiplications | $O(N)$ reference buffer | 100% Parallel across target tracks |
| **Cross-Residual Subtraction**| $(M - 1) \cdot N$ integer MACs | $O(N)$ buffer | 100% Parallel across target tracks |
| **Single Track Decoding** | $2 N \cdot P$ ops (Track $t$ + Ref Track 0) | $O(N)$ buffer | Constant $O(1)$ overhead |

**Decoding Overheads:**
- Decoding all $M$ tracks: Requires decoding Track 0 once, then decoding Tracks $1..M-1$ in parallel using Track 0's cached residual.
- Decoding 1 target track $t$: Requires decoding Track 0's residual + Track $t$'s residual $\implies$ exact $2\times$ single-track work, maintaining instant playhead scrubbing.

---

## 6. Memory Analysis

- **Encoder Buffer Workspace:**

  $$\text{Memory} = M \times N \times 8 \text{ bytes} = 32 \times 4096 \times 8 = 1.048 \text{ MB}$$

  Fits easily within CPU L3 cache ($16-64 \text{ MB}$).

- **Decoder Workstation Memory:**
  When extracting a single track $t$, only two residual vectors are maintained in RAM ($E_0$ and $E_t$), requiring only $2 \times 4096 \times 8 = 65.5 \text{ KB}$.

---

## 7. Comparison with Existing Codecs

| Codec | Max Channels | Inter-Channel Decorrelation Strategy | Random Access Single Track Latency |
| :--- | :--- | :--- | :--- |
| **FLAC (RFC 9639)** | 8 channels | Fixed 2-channel modes (L/R, L/S, R/S, M/S) | Instant $O(1)$ |
| **WavPack** | 256 channels | Pairwise adjacent channel subtraction | $O(k)$ where $k$ is dependency chain length |
| **MPEG-4 ALS** | 65536 channels | Full matrix prediction / sequential ordering | $O(M)$ (must decode prior channels) |
| **Loom** | Unlimited tracks | Star-graph / 1-hop DAG cross-track prediction | **Guaranteed $O(1)$ (max 2 tracks decoded)** |

---

## 8. Implementation Strategy

1. **Reference Track Selection:**  
   Loom designates **Track 0** (the primary FLAC master mix) as the root reference node.
2. **Bitstream Encoding:**  
   In internal `.loom` subframes, a 1-bit flag `has_ref_track` indicates cross-prediction. If set, it is followed by `ref_track_idx` (2 bytes) and `ref_weight` (1 byte signed `i8`).
3. **Rust SIMD Optimization:**  
   Cross-track covariance and subtraction use 256-bit AVX2 SIMD intrinsics.

---

## 9. Rust-Specific Considerations

### 9.1 AVX2 Accelerated Cross-Track Subtraction
```rust
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[inline]
pub unsafe fn apply_cross_prediction_avx2(
    target_res: &mut [i64],
    ref_res: &[i64],
    weight_q8: i8,
) {
    let n = target_res.len();
    let weight = _mm256_set1_epi64x(weight_q8 as i64);
    
    let mut i = 0;
    while i + 4 <= n {
        let target_vec = _mm256_loadu_si256(target_res.as_ptr().add(i) as *const _);
        let ref_vec = _mm256_loadu_si256(ref_res.as_ptr().add(i) as *const _);
        
        // Multiply: (ref_vec * weight)
        let prod = _mm256_mullo_epi64(ref_vec, weight);
        // Shift right by 8 bits
        let pred = _mm256_srai_epi64(prod, 8);
        // Subtract prediction: target_vec - pred
        let res = _mm256_sub_epi64(target_vec, pred);
        
        _mm256_storeu_si256(target_res.as_mut_ptr().add(i) as *mut _, res);
        i += 4;
    }
    
    // Scalar fallback for remaining elements
    for j in i..n {
        let pred = (ref_res[j] * weight_q8 as i64) >> 8;
        target_res[j] -= pred;
    }
}
```

---

## 10. Benchmark Methodology

### 10.1 Multi-track Stem Test Suite
1. **24-Track Rock Session (Drums, Bass, Guitars, Vocals):** 24-bit / 48kHz.
2. **48-Track Orchestral Session (Strings, Brass, Woodwinds, Percussion):** 24-bit / 96kHz.

### 10.2 Target Metrics
- **Multitrack Compression Ratio Advantage:** $\ge 12\%$ smaller file size compared to 24 individual FLAC files compressed independently.
- **Single-Track Seeking Overhead:** $< 1.5\times$ latency compared to standalone FLAC.

---

## 11. References

1. **Openshaw, D. (2002):** *WavPack Transitional Lossless Audio Compression.* WavPack Documentation.
2. **Liebchen, T. (2006):** *MPEG-4 Audio Lossless Coding (ALS).* ISO/IEC JTC1/SC29/WG11 N6435.
3. **Gersho, A., Gray, R. M. (1992):** *Vector Quantization and Signal Compression.* Kluwer Academic Publishers.

---

## 12. Open Research Questions

1. **Multi-Reference Graph Optimization:** Can a 2-reference predictor ($\hat{E}_t = w_1 E_A + w_2 E_B$) provide additional entropy savings without exceeding the 2-track single-track extraction threshold?

---

## 13. Future Improvements

- **Adaptive Pitch-Shifted Coupling:** Incorporate fractional delay filters into cross-track prediction to account for micro-timing differences between multi-mic acoustic setups.
