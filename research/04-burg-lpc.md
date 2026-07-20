# Research Paper 04: Linear Prediction Analysis: Burg's Method, Levinson-Durbin Recursion, and Lattice Filters

**Author:** Loom Codec Research Group  
**Status:** Complete  
**Date:** July 2026  
**Sources:** [RFC 9639 §4.3](https://www.rfc-editor.org/rfc/rfc9639.html), Burg (1967), Makhoul (1975), Levinson (1947), Durbin (1960)

---

## 1. Problem Statement

In lossless audio compression, linear prediction reduces the dynamic range and variance of audio signals by subtracting a linear combination of previous samples $\hat{x}[n] = \sum_{i=1}^{p} a_i x[n-i]$ from the target sample $x[n]$. The resulting residual sequence $e[n] = x[n] - \hat{x}[n]$ exhibits a probability density function concentrated near zero, which can be encoded at significantly lower bitrates using entropy coding techniques (such as Golomb-Rice coding).

The fundamental challenge in adaptive Linear Predictive Coding (LPC) is estimating the optimal coefficient vector $\mathbf{a} = [a_1, a_2, \dots, a_p]^T$ for a frame of $N$ audio samples. Traditional implementations rely on the **Autocorrelation Method** solved via the **Levinson-Durbin Recursion**. However, windowing artifacts (such as spectral leakage introduced by Hann or Tukey windows) and numerical instability in high-order models ($p > 12$) often lead to sub-optimal prediction residuals or unstable synthesis filters ($|k_i| \ge 1$).

This paper investigates alternative linear prediction algorithms, specifically **Burg's Maximum Entropy Method (MEM)**, the **Covariance Method**, and **Lattice Synthesis Filters**, evaluating their mathematical foundations, stability guarantees, computational complexity, and applicability to Loom's Rust-based compression engine.

---

## 2. Historical Background

The foundation of linear prediction in discrete-time signals dates back to the work of Norbert Wiener (1949) and Andrey Kolmogorov (1941) on time-series extrapolation. In 1947, Norman Levinson formulated an efficient algorithm for solving Toeplitz systems of linear equations, which was later simplified by James Durbin (1960) for autoregressive model fitting.

In 1967, John Parker Burg introduced the **Maximum Entropy Method** (Burg's algorithm) for geophysical seismic signal processing. Unlike the autocorrelation method, which assumes the signal is zero outside the observed window (creating artificial discontinuities at boundaries), Burg's method minimizes the sum of forward and backward prediction error energies directly on the unwindowed data segment.

During the late 1970s and 1980s, LPC became the dominant paradigm in speech compression (FS-1015 LPC-10, CELP) and was subsequently adopted by lossless audio codecs in the 1990s:
- **Shorten (1994, Robinson):** Introduced low-order polynomial fixed predictors and low-order Levinson-Durbin LPC.
- **FLAC (2000, Coalson):** Utilized Levinson-Durbin recursion up to order 32 with Hann windowing and quantized integer coefficients.
- **TAK (2007, Becker):** Incorporated higher-order adaptive prediction and advanced lattice structures.
- **WavPack (2002, Openshaw):** Implemented adaptive LMS (Least Mean Squares) finite-impulse-response filters for dynamic tracking.

---

## 3. Mathematical Derivation

### 3.1 The Autoregressive (AR) Model

An autoregressive process of order $p$, denoted $\text{AR}(p)$, models a discrete-time signal $x[n]$ as:
$$x[n] = -\sum_{i=1}^{p} a_i x[n-i] + e[n]$$
where $e[n]$ is a zero-mean white noise excitation process with variance $\sigma_e^2$, and $a_i$ are the predictor coefficients.

The transfer function of the synthesis filter $H(z)$ is given by:
$$H(z) = \frac{1}{A(z)} = \frac{1}{1 + \sum_{i=1}^{p} a_i z^{-i}}$$

### 3.2 Yule-Walker Equations (Autocorrelation Method)

Multiplying both sides of the AR equation by $x[n-k]$ and taking the expected value yields the Yule-Walker equations:
$$R_{xx}[k] + \sum_{i=1}^{p} a_i R_{xx}[k-i] = 0, \quad k = 1, 2, \dots, p$$
where $R_{xx}[k] = E\{x[n]x[n-k]\}$ is the autocorrelation function. In matrix form:
$$\begin{bmatrix}
R[0] & R[1] & \dots & R[p-1] \\
R[1] & R[0] & \dots & R[p-2] \\
\vdots & \vdots & \ddots & \vdots \\
R[p-1] & R[p-2] & \dots & R[0]
\end{bmatrix}
\begin{bmatrix}
a_1 \\ a_2 \\ \vdots \\ a_p
\end{bmatrix}
= -\begin{bmatrix}
R[1] \\ R[2] \\ \vdots \\ R[p]
\end{bmatrix}$$

Because the autocorrelation matrix $\mathbf{R}_p$ is Symmetric Toeplitz, the **Levinson-Durbin algorithm** solves this system in $O(p^2)$ operations rather than $O(p^3)$ Gaussian elimination:

$$\begin{aligned}
k_m &= -\frac{R[m] + \sum_{j=1}^{m-1} a_{j}^{(m-1)} R[m-j]}{E^{(m-1)}} \\
a_m^{(m)} &= k_m \\
a_j^{(m)} &= a_j^{(m-1)} + k_m a_{m-j}^{(m-1)}, \quad j = 1, \dots, m-1 \\
E^{(m)} &= E^{(m-1)} (1 - k_m^2)
\end{aligned}$$

where $k_m$ represents the $m$-th **reflection coefficient** (or PARCOR coefficient, Partial Autocorrelation), and $E^{(m)}$ is the forward prediction error energy at iteration $m$.

### 3.3 Burg's Algorithm Formulation

Burg's method avoids computing the autocorrelation matrix explicitly and does not apply windowing. It defines the forward prediction error $f_m[n]$ and backward prediction error $b_m[n]$ for an order $m$ predictor at sample index $n$:
$$f_m[n] = f_{m-1}[n] + k_m b_{m-1}[n-1]$$
$$b_m[n] = b_{m-1}[n-1] + k_m f_{m-1}[n]$$

Base cases ($m=0$):
$$f_0[n] = b_0[n] = x[n], \quad n = 0, 1, \dots, N-1$$

Burg calculates the reflection coefficient $k_m$ by minimizing the sum of forward and backward residual energies for order $m$:
$$\mathcal{E}_m = \sum_{n=m}^{N-1} \left( f_m^2[n] + b_m^2[n] \right)$$

Differentiating $\mathcal{E}_m$ with respect to $k_m$ and setting $\frac{\partial \mathcal{E}_m}{\partial k_m} = 0$ yields:
$$k_m = -\frac{2 \sum_{n=m}^{N-1} f_{m-1}[n] b_{m-1}[n-1]}{\sum_{n=m}^{N-1} \left( f_{m-1}^2[n] + b_{m-1}^2[n-1] \right)}$$

---

## 4. Algorithm Explanation

```
       +---------------------------------------------------+
       | Input PCM Frame: x[0..N-1], Max LPC Order: P_max   |
       +---------------------------------------------------+
                                 |
                                 v
                 Initialize f_0[n] = b_0[n] = x[n]
                                 |
                                 v
                     For order m = 1 to P_max:
       +---------------------------------------------------+
       | 1. Compute reflection coefficient k_m via Burg    |
       |    formula (Cauchy-Schwarz guaranteed |k_m| < 1)  |
       | 2. Update forward error f_m[n] and backward error |
       |    b_m[n] for n = m to N-1                        |
       | 3. Update polynomial coefficients a_j^(m) via     |
       |    Levinson recursion                             |
       | 4. Compute residual entropy / bit estimate        |
       +---------------------------------------------------+
                                 |
                                 v
        Select optimal order m* minimizing total bit cost
                                 |
                                 v
        Quantize coefficients -> Integer bitstream encoding
```

### Key Differences: Burg vs. Levinson-Durbin
1. **Windowing Requirement:** Levinson-Durbin requires windowing (e.g., Hann window) to prevent spectral distortion at frame edges, which alters the original signal energy. Burg operates directly on raw, unwindowed PCM samples.
2. **Stability Guarantee:** By the Cauchy-Schwarz inequality, the denominator in Burg's formula is always strictly greater than or equal to the absolute value of the numerator. Thus, $|k_m| < 1$ is guaranteed for every iteration $m$, ensuring **100% stable synthesis filters**.
3. **Phase & Transient Response:** Burg captures high-frequency transients and sharp pitch boundaries with lower order $p$ than Levinson-Durbin.

---

## 5. Complexity Analysis

Let $N$ be the frame block size (e.g., $N=4096$) and $P$ be the maximum LPC order (e.g., $P=32$).

| Metric | Levinson-Durbin (with Windowing) | Burg's Method | Covariance Method |
| :--- | :--- | :--- | :--- |
| **Windowing Overhead** | $O(N)$ multiplications | None ($0$) | None ($0$) |
| **Autocorrelation / Vector** | $O(N \cdot P)$ ops | None ($0$) | $O(N \cdot P)$ ops |
| **Coefficient Iteration** | $O(P^2)$ ops | $O(N \cdot P)$ per order $\implies O(N \cdot P)$ total | $O(P^3)$ matrix solve |
| **Overall Time Complexity** | $\mathcal{O}(N \cdot P + P^2)$ | $\mathcal{O}(N \cdot P)$ | $\mathcal{O}(N \cdot P + P^3)$ |
| **Filter Stability** | Conditional ($|k_m| < 1$ requires windowing) | Unconditionally Guaranteed ($|k_m| < 1$) | Unstable (requires post-stabilization) |

For standard audio frame sizes ($N = 4096, P = 32$):
- Levinson-Durbin FLAC pipeline: $\approx 4096 \times 32 + 1024 = 132,096$ floating-point ops.
- Burg pipeline: $\approx 2 \times 4096 \times 32 = 262,144$ floating-point ops.

Burg requires approximately $2\times$ the operations of Levinson-Durbin, but eliminates stability check fallbacks and windowing distortions.

---

## 6. Memory Analysis

Burg's algorithm maintains two workspace vectors for forward and backward prediction errors:
- $f[n] \in \mathbb{R}^N$ (Forward error vector)
- $b[n] \in \mathbb{R}^N$ (Backward error vector)
- $a[m] \in \mathbb{R}^P$ (Current LPC coefficients)

**Memory Footprint:**
- Stack allocation for $N = 4096$ with `f64` precision:
  $$\text{Memory} = 2 \times 4096 \times 8 \text{ bytes} + 32 \times 8 \text{ bytes} \approx 65.5 \text{ KB}$$
- For multitrack encoding with $M$ parallel channels (e.g., $M = 32$ stems), memory buffers can be pre-allocated per thread worker, avoiding dynamic heap allocation during the encoding loop.

---

## 7. Comparison with Existing Codecs

| Codec | Prediction Algorithm | Max Order | Window Type | Coefficient Precision | Stability Enforcement |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **libFLAC** | Levinson-Durbin | 32 | Hann / Welch | 5–15 bits (quantized) | Fallback to Fixed if $|k_i| \ge 1$ |
| **WavPack** | Adaptive LMS + Fixed | Variable | None | Adaptive 8-bit | Inherently stable LMS steps |
| **Monkey's Audio (APE)** | High-order adaptive | Up to 256 | Custom | 16-bit fixed point | Complex predictor state mapping |
| **TAK** | Levinson-Durbin / Burg hybrid | 64 | Adaptive Tukey | Variable | Reflection coefficient clamping |
| **Loom (Proposed)** | Burg MEM + Levinson Dual-Engine | 32 | Unwindowed Burg / Hann L-D | 5–15 bits (FLAC RFC 9639 compliant) | Automatic Burg fallback |

---

## 8. Implementation Strategy

Loom will implement a **Dual-Engine Predictor Search**:
1. **Engine A (Fast Mode / Levinson-Durbin):** Computes Hann-windowed autocorrelation and Levinson-Durbin coefficients. If the resulting reflection coefficients satisfy $|k_i| < 0.999$, the model is evaluated.
2. **Engine B (High-Efficiency Mode / Burg MEM):** Runs Burg's algorithm directly on unwindowed PCM. Guaranteed to produce $|k_i| < 1.0$.
3. **Entropy Comparison:** Computes the bit-cost estimate for both methods:
   $$\text{Cost} = \sum_{i=p}^{N-1} \text{RiceBits}(e[i], k_{\text{opt}}) + p \cdot \text{qlp-precision}$$
   The engine selects whichever predictor yields the smaller bit representation.

---

## 9. Rust-Specific Considerations

### 9.1 Memory Alignment & Auto-Vectorization
Burg's error update loop is highly amenable to SIMD vectorization (AVX2 / NEON):
```rust
// Idiomatic Rust vector update optimized for SIMD auto-vectorization
pub fn update_burg_errors(
    f: &mut [f64],
    b: &mut [f64],
    k: f64,
    m: usize,
    n_samples: usize,
) {
    debug_assert_eq!(f.len(), b.len());
    let f_slice = &mut f[m..n_samples];
    let b_slice = &mut b[m..n_samples];
    
    // Using chunked loops to allow LLVM to unroll and auto-vectorize with FMA instructions
    for (f_val, b_val) in f_slice.iter_mut().zip(b_slice.iter_mut()) {
        let f_prev = *f_val;
        let b_prev = *b_val;
        *f_val = f_prev + k * b_prev;
        *b_val = b_prev + k * f_prev;
    }
}
```

### 9.2 Zero-Allocation Buffer Pools
To conform to high-performance real-time bounds, workspace buffers (`Vec<f64>`) are allocated once per thread inside an encoding context pool `LpcWorkspacePool`:
```rust
pub struct BurgWorkspace {
    pub f: Vec<f64>,
    pub b: Vec<f64>,
    pub a: Vec<f64>,
}
```

---

## 10. Benchmark Methodology

### 10.1 Datasets
Testing should be conducted across standard uncompressed 24-bit / 96kHz and 16-bit / 44.1kHz audio corpora:
1. **SQAM (Sound Quality Assessment Material):** Solo instruments (flute, harpsichord, violin), vocal, speech.
2. **24/96 Orchestral / Acoustic Stems:** High dynamic range multi-mic classical sessions.
3. **Modern Electronic & Rock Stems:** Heavy synthetic transients, distorted bass, dense percussion.

### 10.2 Metrics
- **Compression Ratio ($CR$):** $CR = \frac{\text{Uncompressed Bytes}}{\text{Compressed Bytes}}$
- **Residual Variance Reduction ($\Delta \sigma^2$):** $10 \log_{10} \frac{\text{Var}(x)}{\text{Var}(e)} \text{ dB}$
- **Predictor Execution Speed:** Megasamples per second ($\text{MS/s}$) on standard hardware (Intel Core i9 / Apple M-series).

---

## 11. References

1. **RFC 9639 (2024):** *FLAC Audio Coding Format.* Internet Engineering Task Force (IETF).
2. **Burg, J. P. (1967):** *Maximum Entropy Spectral Analysis.* Proceedings of the 37th Meeting of the Society of Exploration Geophysicists (SEG).
3. **Makhoul, J. (1975):** *Linear Prediction: A Tutorial Review.* Proceedings of the IEEE, Vol. 63, No. 4, pp. 561-580.
4. **Durbin, J. (1960):** *The Fitting of Time-Series Models.* Revue de l'Institut International de Statistique, pp. 233-244.
5. **Robinson, T. (1994):** *SHORTEN: Simple Lossless and Near-Lossless Audio Compression.* Technical Report CUED/F-INFENG/TR.156, Cambridge University Engineering Department.

---

## 12. Open Research Questions

1. **Sub-block Burg Adaptation:** Can Burg's algorithm be applied to non-stationary transients using variable sub-block boundaries (e.g., splitting a 4096-sample frame into 512-sample regions dynamically) without increasing coefficient overhead beyond the entropy savings?
2. **Fixed-Point Burg Recursion:** Can Burg's estimation be performed entirely in 32-bit/64-bit integer arithmetic without IEEE 754 floating-point operations while maintaining $|k_m| < 1$ guarantees?

---

## 13. Future Improvements

- **Warped Linear Prediction (WLP):** Incorporate psychoacoustic frequency warping (All-pass filters $z^{-1} \to \frac{z^{-1} - \lambda}{1 - \lambda z^{-1}}$) into Burg's estimation to allocate higher prediction resolution to low-frequency audio bands where human hearing is most sensitive.
- **Sparse LPC Selection:** Implement $L_1$-norm regularization (Lasso LPC) to prune insignificant predictor coefficients to zero, saving coefficient storage bits in the bitstream.
