# Research Paper 22: Adaptive Filter Algorithms for Non-Stationary Audio Prediction: Recursive Least Squares (RLS), Normalized LMS (NLMS), and Kalman Filtering in Lossless Audio Codecs

**Author:** Loom Codec Research Group  
**Status:** Complete  
**Date:** July 2026  
**Sources:** Haykin (2014), Widrow & Stearns (1985), Sayed (2008), Kalman (1960)

---

## 1. Problem Statement

Linear Predictive Coding (LPC) in standard lossless audio codecs (such as FLAC and ALAC) assumes local wide-sense stationarity over fixed frame durations ($N = 512 \dots 4096$). Within each frame, optimal predictor coefficients are estimated once using autocorrelation or Burg methods and transmitted as header overhead.

However, non-stationary audio signals (such as fast vocal vibrato, polyphonic acoustic guitar, or complex percussion) exhibit rapid time-varying spectral properties. Static per-frame LPC coefficients introduce sub-optimal prediction residuals during intra-frame transients.

This paper investigates **Adaptive Filtering** algorithms (Normalized Least Mean Squares (NLMS), Recursive Least Squares (RLS), and Kalman Filtering) that update predictor weights continuously at sample resolution ($n = 1, 2, \dots, N$) without requiring per-frame coefficient transmission in the bitstream.

---

## 2. Historical Background

Adaptive filter theory originated with Widrow and Hoff (1960) through the Least Mean Squares (LMS) algorithm, which applied stochastic gradient descent to approximate Wiener filter solutions. 

In 1974, Plackett and Kalman developed exact recursive matrix inversions leading to the Recursive Least Squares (RLS) algorithm, achieving fast convergence at the cost of $\mathcal{O}(p^2)$ computational complexity per sample.

Lossless audio codecs such as Shorten (Robinson 1994) and FLAC (Coalson 2000) rejected sample-adaptive filtering in favor of block-based LPC due to the CPU limitations of 1990s processors. Modern multi-gigahertz AVX2/AVX-512 processors enable real-time sample-by-sample adaptive weight adaptation, making adaptive filtering viable for high-density multitrack codecs.

---

## 3. Mathematical Derivation

### 3.1 Normalized Least Mean Squares (NLMS)

Let $\mathbf{x}[n] = [x[n-1], x[n-2], \dots, x[n-p]]^T$ denote the vector of $p$ previous audio samples. The predicted sample $\hat{x}[n]$ and prediction error $e[n]$ are given by:

$$\hat{x}[n] = \mathbf{w}^T[n] \mathbf{x}[n]$$

$$e[n] = x[n] - \hat{x}[n]$$

The coefficient vector $\mathbf{w}[n]$ is updated at each sample using the normalized step size $\mu$:

$$\mathbf{w}[n+1] = \mathbf{w}[n] + \frac{\mu}{\varepsilon + \|\mathbf{x}[n]\|^2} e[n] \mathbf{x}[n]$$

where $\varepsilon > 0$ prevents division by zero in silent audio regions.

### 3.2 Recursive Least Squares (RLS)

RLS minimizes the exponentially weighted cost function:

$$\mathcal{E}[n] = \sum_{i=1}^{n} \lambda^{n-i} \left( x[i] - \mathbf{w}^T[n] \mathbf{x}[i] \right)^2$$

where $\lambda \in (0, 1]$ is the forgetting factor.

The exact RLS gain vector $\mathbf{k}[n]$ and inverse correlation matrix $\mathbf{P}[n] = \mathbf{\Phi}^{-1}[n]$ update equations are:

$$\mathbf{k}[n] = \frac{\lambda^{-1} \mathbf{P}[n-1] \mathbf{x}[n]}{1 + \lambda^{-1} \mathbf{x}^T[n] \mathbf{P}[n-1] \mathbf{x}[n]}$$

$$\mathbf{w}[n] = \mathbf{w}[n-1] + \mathbf{k}[n] e[n]$$

$$\mathbf{P}[n] = \lambda^{-1} \mathbf{P}[n-1] - \lambda^{-1} \mathbf{k}[n] \mathbf{x}^T[n] \mathbf{P}[n-1]$$

---

## 4. Algorithm Explanation

```
Algorithm: Sample-Adaptive RLS Predictor

Input: Audio sample sequence x[1..N], predictor order p, forgetting factor lambda, initial P scale delta
Output: Residual sequence e[1..N]

1. Initialize w = [0, 0, ..., 0]^T
2. Initialize P = delta * I_{p x p}
3. For n = 1 to N:
     a. Form vector x_vec = [x[n-1], ..., x[n-p]]^T
     b. Compute prediction x_hat = dot(w, x_vec)
     c. Compute residual e[n] = x[n] - round(x_hat)
     d. Compute numerator pi_vec = P * x_vec
     e. Compute scalar denom = lambda + dot(x_vec, pi_vec)
     f. Compute gain k_vec = pi_vec / denom
     g. Update weights w = w + k_vec * e[n]
     h. Update P = (P - outer_product(k_vec, x_vec^T * P)) / lambda
4. Return e[1..N]
```

---

## 5. Complexity Analysis

- **NLMS:** $\mathcal{O}(p)$ additions and multiplications per sample. Space complexity is $\mathcal{O}(p)$.
- **RLS:** $\mathcal{O}(p^2)$ multiplications per sample due to matrix-vector multiplication $\mathbf{P}[n-1] \mathbf{x}[n]$. Space complexity is $\mathcal{O}(p^2)$.
- **Block LPC (Levinson-Durbin):** $\mathcal{O}(N \cdot p + p^2)$ per block of $N$ samples. For $N = 4096$, per-sample amortized complexity is $\mathcal{O}(p + p^2/N) \approx \mathcal{O}(p)$.

---

## 6. Memory Analysis

- **NLMS State:** $p$ 64-bit float weights + $p$ sample buffer = $16p$ bytes.
- **RLS State:** $p^2$ 64-bit covariance matrix + $p$ weights + $p$ buffer = $8p^2 + 16p$ bytes. For order $p = 16$, state fits inside 2.2 KB of L1 cache.

---

## 7. Comparison with Existing Codecs

| Codec | Prediction Type | Per-Sample Weight Update | Coefficient Header Overhead | Transient Tracking |
|-------|-----------------|--------------------------|----------------------------|--------------------|
| FLAC | Block LPC / Fixed | No | 12 to 15 bits per order | Poor |
| WavPack | Fast Adaptive LMS | Yes (NLMS order 1 to 3) | 0 bits | Good |
| TAK | Block LPC | No | 10 to 14 bits per order | Fair |
| Loom (Proposed) | Hybrid RLS / Burg | Optional Sample-Adaptive RLS | 0 bits in RLS mode | Superior |

---

## 8. Implementation Strategy

In Loom, sample-adaptive RLS is evaluated as a candidate predictor alongside Burg LPC during encoder search:
1. Initialize deterministic initial state $\mathbf{w} = \mathbf{0}$, $\mathbf{P} = 100 \cdot \mathbf{I}$.
2. Run RLS predictor across the block, deriving integer residuals $e[n] = x[n] - \lfloor \mathbf{w}^T[n] \mathbf{x}[n] \rceil$.
3. Because both encoder and decoder execute the exact same deterministic RLS update equations, zero filter coefficients need to be transmitted in the frame header.

---

## 9. Rust-Specific Considerations

1. **Deterministic Floating-Point Arithmetic:** To prevent decoder divergence across x86_64 (AVX2) and AArch64 (NEON), fixed-point 64-bit integer matrix math or strict IEEE 754 float operations without fast-math compiler optimizations (`-C target-cpu=native` unsafe flags) must be enforced.
2. **Cache Alignment:** Align the covariance matrix $\mathbf{P}$ to 64-byte boundaries using `#[repr(align(64))]` for SIMD matrix updates.

---

## 10. Benchmark Methodology

- **Dataset:** Non-stationary acoustic solo guitar, speech plosives, and polyphonic piano stems from EBU SQAM corpus.
- **Metrics:** Residual entropy $H(E)$, encoding time (MB/s), decoding time (MB/s).
- **Target:** Achieve $0.15 \text{ bits/sample}$ lower entropy on non-stationary audio compared to order-12 static LPC.

---

## 11. References

1. **Haykin, S. (2014):** *Adaptive Filter Theory.* Pearson, 5th Edition.
2. **Widrow, B., Stearns, S. D. (1985):** *Adaptive Signal Processing.* Prentice-Hall.
3. **Sayed, A. H. (2008):** *Adaptive Filters.* John Wiley & Sons.
4. **Kalman, R. E. (1960):** *A New Approach to Linear Filtering and Prediction Problems.* Journal of Basic Engineering, Vol. 82, No. 1, pp. 35-45.

---

## 12. Open Research Questions

- Can fixed-point 32-bit/64-bit integer arithmetic replace double-precision float operations in RLS matrix inversion without numerical drift?
- What is the optimal forgetting factor $\lambda$ for high-sample-rate (96 kHz) multitrack audio?

---

## 13. Future Improvements

- Implement Fast Transversal Filters (FTF) to reduce RLS computational complexity from $\mathcal{O}(p^2)$ to $\mathcal{O}(p)$ per sample.
- Integrate sparse matrix representation for off-diagonal covariance terms in multitrack cross-channel prediction.
