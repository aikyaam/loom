# Research Paper 10: Audio Transform Analysis: Reversible Integer Transforms (IntMDCT, Integer Wavelets) vs. Time-Domain Linear Prediction in Bit-Exact Lossless Compression

**Author:** Loom Codec Research Group  
**Status:** Complete  
**Date:** July 2026  

---

## 1. Problem Statement

A fundamental decision in audio compression architecture is choosing between **Time-Domain Predictive Coding** (such as Linear Predictive Coding, LPC) and **Frequency-Domain Transform Coding** (such as Discrete Fourier Transform, DFT; Modified Discrete Cosine Transform, MDCT; or Discrete Wavelet Transform, DWT).

In lossy audio codecs (such as AAC, MP3, Opus, Opus/Vorbis), transform coding dominates because human auditory perception (psychoacoustics) operates in the time-frequency domain via critical band masking. However, applying transform coding to **lossless, bit-exact compression** introduces severe theoretical and practical barriers:
1. **Irrational Coefficients:** Standard MDCT/FFT kernels contain trigonometric constants ($\cos\frac{\pi k}{N}$, $\sin\frac{\pi k}{N}$) requiring floating-point arithmetic. Floating-point rounding is non-deterministic across hardware architectures (x86 vs. ARM vs. RISC-V), violating bit-exact reproducibility.
2. **Reversibility Constraints:** Forward transform $\mathbf{X} = \mathbf{T} \mathbf{x}$ and inverse transform $\mathbf{x} = \mathbf{T}^{-1} \mathbf{X}$ must map $\mathbb{Z}^N \to \mathbb{Z}^N$ dynamically without precision loss or dynamic range explosion.

This paper evaluates **Reversible Integer MDCT (IntMDCT)**, **Integer Wavelet Transforms (S+P Transform, Lifting Schemes)**, and **Time-Domain Linear Prediction**, detailing why time-domain LPC remains superior for lossless audio codecs while defining specific multitrack scenarios where integer transforms provide measurable entropy gains.

---

## 2. Historical Background

- **Time-Domain LPC Superiority (1990s):** Codecs like Shorten, FLAC, WavPack, and ALAC chose time-domain linear prediction due to its mathematical simplicity, deterministic integer arithmetic, low computational complexity, and natural adaptation to non-stationary audio signals.
- **IntMDCT Invention (1999–2002):** Geiger, Herre, Yu, and Rahardja introduced the **Integer Modified Discrete Cosine Transform (IntMDCT)** using Givens rotation lifting steps to map integer PCM values to integer transform coefficients losslessly.
- **Integer Wavelets (1996–1998):** Sweldens introduced the **Lifting Scheme**, proving that any Discrete Wavelet Transform can be decomposed into reversible integer-to-integer mapping steps ($S+P$ transform, Cohen-Daubechies-Feauveau wavelets).
- **Hybrid Codecs (MPEG-4 ALS / Audio Lossless Coding):** ISO/IEC 14496-5 (MPEG-4 ALS) integrated both high-order LPC and optional IntMDCT to maximize compression for complex orchestral audio at the cost of significantly higher computational complexity.

---

## 3. Mathematical Derivation

### 3.1 Standard MDCT Formulation

For a frame of $2N$ samples $x[n]$, the MDCT produces $N$ frequency coefficients $X[k]$:
$$X[k] = \sum_{n=0}^{2N-1} h[n] x[n] \cos\left[ \frac{\pi}{N} \left( n + \frac{1}{2} + \frac{N}{2} \right) \left( k + \frac{1}{2} \right) \right], \quad k = 0, 1, \dots, N-1$$
where $h[n]$ is a smooth window function (e.g., Sine or Kaiser-Bessel Derived window) satisfying the Princen-Bradley condition:
$$h^2[n] + h^2[n+N] = 1$$

Because the matrix multiplication uses real-valued floats, direct inverse transformation $\mathbf{x} = \text{IMDCT}(\mathbf{X})$ yields small rounding errors ($\approx 10^{-7}$), making standard MDCT inherently lossy.

### 3.2 Lifting Scheme & Reversible Integer Givens Rotations

To construct an integer-to-integer reversible transform, any $2\times 2$ Givens rotation matrix $\mathbf{R}(\theta) = \begin{bmatrix} \cos\theta & -\sin\theta \\ \sin\theta & \cos\theta \end{bmatrix}$ is decomposed into three structural **lifting steps**:

$$\begin{bmatrix} \cos\theta & -\sin\theta \\ \sin\theta & \cos\theta \end{bmatrix} = 
\begin{bmatrix} 1 & \frac{\cos\theta - 1}{\sin\theta} \\ 0 & 1 \end{bmatrix}
\begin{bmatrix} 1 & 0 \\ \sin\theta & 1 \end{bmatrix}
\begin{bmatrix} 1 & \frac{\cos\theta - 1}{\sin\theta} \\ 0 & 1 \end{bmatrix}$$

Let $a = \frac{\cos\theta - 1}{\sin\theta} = -\tan\frac{\theta}{2}$ and $b = \sin\theta$.

By inserting integer rounding operators $\lfloor \cdot \rceil$ after each lifting step, the transformation maps $\mathbb{Z}^2 \to \mathbb{Z}^2$ **without any loss of information**:

$$\begin{aligned}
x_1' &= x_1 + \lfloor a \cdot x_2 \rceil \\
x_2' &= x_2 + \lfloor b \cdot x_1' \rceil \\
x_1'' &= x_1' + \lfloor a \cdot x_2' \rceil
\end{aligned}$$

**Exact Inverse Operation (Reconstruction):**
$$\begin{aligned}
x_1' &= x_1'' - \lfloor a \cdot x_2' \rceil \\
x_2 &= x_2' - \lfloor b \cdot x_1' \rceil \\
x_1 &= x_1' - \lfloor a \cdot x_2 \rceil
\end{aligned}$$

Because subtraction cancels addition exactly regardless of rounding errors inside $\lfloor \cdot \rceil$, the inverse operation restores $(x_1, x_2) \in \mathbb{Z}^2$ bit-identically!

---

## 4. Algorithm Explanation

```
       +---------------------------------------------------+
       |            PCM Audio Samples x[n] in Z            |
       +---------------------------------------------------+
                                 |
         +-----------------------+-----------------------+
         |                                               |
         v                                               v
Time-Domain Path (LPC)                       Integer Transform Path (IntMDCT)
         |                                               |
  1. Compute Autocorrelation                      1. Apply DCT-IV via Givens
  2. Solve Levinson-Durbin                           Lifting Steps in Z
  3. Predict x̂[n] = sum(a_i * x[n-i])            2. Integer Rounding after
  4. Compute residual e[n] = x[n] - x̂[n]             each stage
         |                                               |
         v                                               v
 Residual PDF: Laplacian                         Frequency Coefficients:
 Peaked around zero in Z                         Peaked around zero in Z
         |                                               |
         +-----------------------+-----------------------+
                                 |
                                 v
                       Entropy Coding Engine
                   (Golomb-Rice or tANS/rANS)
```

### 4.1 Why Time-Domain LPC Outperforms IntMDCT for General Lossless Audio

1. **Transient Performance & Bit Expansion:**  
   IntMDCT spreads localized sharp transients (e.g., drum attacks, foley hits) across all $N$ frequency bins (spectral smearing). In LPC, a transient creates high residuals for only a few samples, leaving the rest of the block near zero.
2. **Coefficient Memory Overhead:**  
   LPC requires storing only $P$ quantized coefficients (where $P \le 32$, taking $\sim 30$ bytes per block). IntMDCT requires storing grouping/scale-factor metadata for $N$ bins (taking $\sim 100-200$ bytes per block), offsetting entropy savings on short frames.
3. **Computational Complexity:**  
   IntMDCT lifting requires $3\times$ the multiplication/rounding steps of an FFT. LPC prediction runs in pure integer MAC (Multiply-Accumulate) instructions.

---

## 5. Complexity Analysis

Let $N = 4096$ be the frame block size and $P = 16$ be the LPC order.

| Method | Forward Transform Ops / Block | Inverse Transform Ops / Block | Dynamic Range Expansion | Bit-Exact Across Architectures |
| :--- | :--- | :--- | :--- | :--- |
| **Fixed Predictor (Order 2)** | $2 N \approx 8,192 \text{ ops}$ | $2 N \approx 8,192 \text{ ops}$ | $+1 \text{ bit}$ | Yes (Deterministic Integer) |
| **Adaptive LPC (Order 16)** | $P \cdot N \approx 65,536 \text{ ops}$ | $P \cdot N \approx 65,536 \text{ ops}$ | $+1 \text{ bit}$ | Yes (Quantized Shift/MAC) |
| **IntMDCT (Lifting Scheme)** | $3 N \log_2 N \approx 147,456 \text{ ops}$ | $3 N \log_2 N \approx 147,456 \text{ ops}$ | $+2 \text{ to } +3 \text{ bits}$ | Yes (Integer Lifting) |
| **Integer Wavelet (S+P)** | $4 N \approx 16,384 \text{ ops}$ | $4 N \approx 16,384 \text{ ops}$ | $+1 \text{ bit}$ | Yes (Integer Lifting) |

---

## 6. Memory Analysis

- **LPC Memory Footprint:** Requires $O(P)$ state memory ($\sim 256$ bytes for order 32). Fits inside CPU registers.
- **IntMDCT Memory Footprint:** Requires storing $2N$ samples for overlapping windows, plus $N$ intermediate lifting states. For $N=4096$, memory requirement is $2 \times 4096 \times 8 = 65.5 \text{ KB}$, triggering cache eviction on low-power embedded processors.

---

## 7. Comparison with Existing Codecs

| Codec | Core Prediction / Transform | Primary Reason for Architecture Selection |
| :--- | :--- | :--- |
| **FLAC** | Time-Domain Fixed + Adaptive LPC | Maximum decode throughput, low memory, deterministic integer math |
| **WavPack** | Time-Domain Adaptive FIR Predictors | Fast playback, hybrid lossy/lossless stream architecture |
| **ALAC** | Time-Domain Adaptive LPC | Simple implementation for iPod hardware processors |
| **MPEG-4 ALS**| Hybrid (Time-Domain LPC + optional IntMDCT) | Maximum possible compression for complex organ/harmonics |
| **Loom** | **Time-Domain LPC (Primary Engine)** | Optimized for real-time DAW playhead scrubbing & multitrack cross-prediction |

---

## 8. Implementation Strategy

Based on this research, **Loom establishes Time-Domain LPC as its primary core engine**, adhering to RFC 9639 FLAC compatibility.

However, for specialized non-audio multitrack metadata or static control curves (e.g., dense MIDI/automation envelopes), Loom provides an experimental **Integer Wavelet (S+P Transform)** mode within internal `.loom` container extensions.

---

## 9. Rust-Specific Considerations

Reversible integer lifting requires strict handling of integer overflow and deterministic rounding:

```rust
/// Reversible Integer Givens Rotation Step in Rust
#[inline(always)]
pub fn reversible_givens_step(x1: i64, x2: i64, alpha_q16: i64, beta_q16: i64) -> (i64, i64) {
    // Lifting step 1: x1' = x1 + round(alpha * x2)
    let x1_prime = x1 + ((alpha_q16 * x2 + 32768) >> 16);
    // Lifting step 2: x2' = x2 + round(beta * x1')
    let x2_prime = x2 + ((beta_q16 * x1_prime + 32768) >> 16);
    // Lifting step 3: x1'' = x1' + round(alpha * x2')
    let x1_double_prime = x1_prime + ((alpha_q16 * x2_prime + 32768) >> 16);
    
    (x1_double_prime, x2_prime)
}

/// Exact Reverse Integer Givens Step
#[inline(always)]
pub fn inverse_reversible_givens_step(x1_dp: i64, x2_p: i64, alpha_q16: i64, beta_q16: i64) -> (i64, i64) {
    let x1_prime = x1_dp - ((alpha_q16 * x2_p + 32768) >> 16);
    let x2 = x2_p - ((beta_q16 * x1_prime + 32768) >> 16);
    let x1 = x1_prime - ((alpha_q16 * x2 + 32768) >> 16);
    (x1, x2)
}
```

---

## 10. Benchmark Methodology

### 10.1 Evaluated Metrics
- **Compression Ratio Difference ($\Delta CR$):** $CR_{\text{IntMDCT}} - CR_{\text{LPC}}$.
- **Decoding CPU Cycles per Sample (Cycles/sample).**

---

## 11. References

1. **Geiger, R., Yu, R., Herre, J. et al. (2002):** *Lossless Audio Coding Using the IntMDCT.* AES 112th Convention, Munich.
2. **Sweldens, W. (1996):** *The Lifting Scheme: A Custom Construction of Second Generation Wavelets.* Studies in Applied Mathematics, Vol. 99, No. 2, pp. 187-217.
3. **ISO/IEC 14496-5:2006/Amd 10:2007:** *MPEG-4 Audio Lossless Coding (ALS).* International Organization for Standardization.
4. **Princen, J., Bradley, A. (1986):** *Analysis/Synthesis Filter Bank Design Based on Time Domain Aliasing Cancellation.* IEEE Transactions on ASSP, Vol. 34, No. 5, pp. 1153-1161.

---

## 12. Open Research Questions

1. **Adaptive Subband Prediction:** Can a 2-band Integer Wavelet filter pre-split audio into Low and High frequency bands before running time-domain LPC to improve high-sample-rate (192kHz) compression?

---

## 13. Future Improvements

- Incorporate integer S+P wavelet decomposition for high-density multi-channel automation parameters stored inside Loom edit metadata blocks.
