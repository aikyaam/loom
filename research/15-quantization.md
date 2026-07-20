# Research Paper 15: Fixed-Point Arithmetic & Quantization Theory: Precision Loss, Dynamic Range, and Integer Operations in Lossless Audio Codecs

**Author:** Loom Codec Research Group  
**Status:** Complete  
**Date:** July 2026  
**Sources:** [RFC 9639 §4.3](https://www.rfc-editor.org/rfc/rfc9639.html), Coalson (2000), Goldberg (1991)

---

## 1. Problem Statement

A core requirement of any lossless audio codec is **100% deterministic bit-exact reconstruction** across heterogeneous computing hardware (x86-64, ARM64, RISC-V, WebAssembly). 

Floating-point operations (`f32` and `f64`) governed by the IEEE 754 standard do not guarantee identical results across different compilers, target instruction sets (e.g., FMA instructions vs. separate multiply and add), or floating-point rounding modes. Small discrepancies in the least significant mantissa bit of an LPC prediction sample will compound across an audio frame, resulting in corrupted output and loss of exact reconstruction.

To eliminate non-determinism, lossless decoders execute prediction and synthesis using **pure integer fixed-point arithmetic**. The fundamental engineering challenge is designing a coefficient quantization and fixed-point execution pipeline that:
1. Prevents integer overflow during intermediate 64-bit Multiply-Accumulate (MAC) calculations for high bit-depth audio (24-bit / 32-bit PCM).
2. Minimizes quantization noise and truncation error introduced when continuous floating-point LPC coefficients $a_i \in \mathbb{R}$ are quantized to $q_i \in \mathbb{Z}$ at precision $Q \in [5, 15]$ bits.
3. Formulates exact bitwise shift and rounding behavior (`>> shift`) guaranteed to execute identically across all hardware architectures.

---

## 2. Historical Background

- **IEEE 754 Non-Determinism (1985):** While IEEE 754 standardizes floating-point representation, differences in compiler optimization flags (`-ffast-math`), intermediate 80-bit x87 register rounding vs. 128-bit SSE2, and fused multiply-add (`fma`) accumulation lead to subtle variations across platforms.
- **Shorten (1994, Robinson):** Employed simple 16-bit integer shift prediction, limiting predictor orders to 4.
- **FLAC (2000, Coalson):** Formalized integer coefficient quantization using a 4-bit precision field ($Q \in [1, 16]$) and a 5-bit signed right-shift parameter ($S \in [-16, 15]$), enabling pure 64-bit integer decoding.
- **WavPack & TAK (2002–2007):** Adopted fixed-point $Q8.8$ and $Q1.15$ accumulator models, enforcing deterministic integer operations across 32-bit and 64-bit CPU architectures.

---

## 3. Mathematical Derivation

### 3.1 Quantization of Predictor Coefficients

Let $a_1, a_2, \dots, a_p \in \mathbb{R}$ be the floating-point LPC coefficients calculated via Levinson-Durbin or Burg's algorithm.

Let $Q$ be the desired quantization precision (bits per coefficient, $5 \le Q \le 15$).  
Let $a_{\max} = \max_{i} |a_i|$ be the maximum absolute coefficient value in the frame.

To maximize bit utilization within the available $Q$-bit range without overflow, we compute an integer shift factor $S \in \mathbb{Z}$:
$$S = \left\lceil \log_2(a_{\max}) \right\rceil + Q - b_{\text{target}}$$
where $b_{\text{target}} = Q - 1$ represents the available magnitude range excluding the sign bit.

The quantized integer coefficients $q_i \in \mathbb{Z}$ are computed as:
$$q_i = \text{round}\left( a_i \cdot 2^{S} \right) = \left\lfloor a_i \cdot 2^{S} + 0.5 \right\rfloor$$

### 3.2 Dynamic Range & Overflow Bound Analysis

During frame decoding, the predicted sample $\hat{x}[n]$ is calculated as:
$$\hat{x}[n] = \left( \sum_{i=1}^{p} q_i \cdot x[n-i] \right) \gg S$$

Let $B$ be the input audio sample bit depth (e.g., $B = 24$ bits for 24-bit audio).  
The sample magnitude is bounded by $|x[n]| \le 2^{B-1}$.  
The quantized coefficient magnitude is bounded by $|q_i| \le 2^{Q-1}$.

The maximum possible absolute sum in the accumulator before shifting is:
$$|\text{Acc}_{\max}| = \sum_{i=1}^{p} |q_i| \cdot |x[n-i]| \le p \cdot (2^{Q-1}) \cdot (2^{B-1}) = p \cdot 2^{B + Q - 2}$$

**Required Bit-Width Calculation:**  
To prevent integer overflow, the accumulator bit-width $W_{\text{acc}}$ must satisfy:
$$W_{\text{acc}} \ge \log_2(p) + B + Q - 1$$

For worst-case studio parameters ($P = 32$ order, $B = 24$ bit depth, $Q = 15$ coefficient precision):
$$W_{\text{acc}} \ge \log_2(32) + 24 + 15 - 1 = 5 + 24 + 15 - 1 = 43 \text{ bits}$$

Since $43 \text{ bits} < 64 \text{ bits}$, performing MAC accumulation using **signed 64-bit integers (`i64`)** guarantees **zero risk of integer overflow** across all legal FLAC/Loom parameter combinations!

---

## 4. Algorithm Explanation

```
                           Floating-Point Coefficients a_i in R
                                          |
                                          v
                              Find Max |a_i| -> a_max
                                          |
                                          v
                      Compute Shift S = ceil(log2(a_max)) + Q - (Q-1)
                                          |
                                          v
                 Quantize q_i = round(a_i * 2^S), Stored as Q-bit Integer
                                          |
                                          v
+-----------------------------------------------------------------------------------+
| Encoder / Decoder Fixed-Point Prediction Loop (Pure Integer i64)                  |
|                                                                                   |
|   1. Initialize acc = 0 (64-bit signed integer)                                   |
|   2. For i = 1 to p:                                                              |
|          acc += (q_i as i64) * (x[n-i] as i64)                                     |
|   3. Apply arithmetic right-shift:                                                |
|          predicted_sample = acc >> S                                              |
|   4. Reconstruct original sample:                                                 |
|          x[n] = residual[n] + predicted_sample                                   |
+-----------------------------------------------------------------------------------+
```

### 4.1 Right-Shift Floor vs. Truncation Behavior
In Rust and C99, right-shifting a negative signed integer (`acc >> S`) performs **arithmetic right shift** (sign-extending the top bits). Arithmetic right shift rounds toward $-\infty$ (floor rounding), whereas integer division (`acc / 2^S`) rounds toward zero (truncation).

FLAC and Loom bitstreams strictly mandate **arithmetic right shift** (floor division):
$$\text{acc} \gg S = \left\lfloor \frac{\text{acc}}{2^S} \right\rfloor$$

This ensures that the predicted sample calculation is single-instruction on x86 (`sar`) and ARM (`asr`), eliminating conditional branches in the decoding loop.

---

## 5. Complexity Analysis

Let $N$ be the frame block size ($N = 4096$) and $P$ be the LPC order ($P = 16$).

| Arithmetic Mode | Instructions / Sample | Clock Cycles / Sample (x86-64) | SIMD Vectorizability | Hardware Portability |
| :--- | :--- | :--- | :--- | :--- |
| **Float (`f64`) Direct** | 1 Multiply, 1 Add | $\sim 4.0$ cycles | High (AVX2 `vfmadd231pd`) | Non-deterministic (IEEE 754 variation) |
| **Fixed-Point (`i32`)** | 1 IMUL, 1 ADD, 1 SAR | $\sim 1.5$ cycles | Outstanding (AVX2 `vpmaddwd`) | Limited to 16-bit audio (overflow risk) |
| **Fixed-Point (`i64` Loom)** | 1 IMUL, 1 ADD, 1 SAR | $\sim 2.0$ cycles | High (AVX2 `vpmuldq`) | **100% Bit-Exact & Deterministic** |

---

## 6. Memory Analysis

- **Coefficient Storage Overhead:**
  Quantized coefficients require $P \times Q$ bits per frame header.
  For order $P = 12$ and precision $Q = 12$:
  $$\text{Storage} = 12 \times 12 = 144 \text{ bits} = 18 \text{ bytes}$$
- **Execution Memory:**
  Zero dynamic heap allocations. Execution requires a single 64-bit register for `acc`.

---

## 7. Comparison with Existing Codecs

| Codec | Max Bit Depth Supported | Predictor Arithmetic | Accumulator Width | Rounding Definition |
| :--- | :--- | :--- | :--- | :--- |
| **FLAC (RFC 9639)** | 32 bits | Fixed-point integer | 64-bit signed (`int64_t`) | Arithmetic Right Shift (`>> S`) |
| **WavPack** | 32 bits (Int/Float) | Fixed-point integer | 64-bit signed | Arithmetic Right Shift |
| **ALAC** | 32 bits | Fixed-point integer | 32-bit / 64-bit adaptive | Arithmetic Right Shift |
| **Loom (Core)** | **32 bits** | **Fixed-point integer (`i64`)** | **64-bit signed (`i64`)** | **Arithmetic Right Shift (`>> S`)** |

---

## 8. Implementation Strategy

Loom implements coefficient quantization in `loom-core/src/predict/lpc.rs`:
1. Calculate floating-point LPC coefficients using Burg or Levinson-Durbin.
2. Determine optimum $Q \in [5, 15]$ by evaluating precision vs. header bit-cost.
3. Quantize coefficients to `i32` values and compute exact shift parameter `qlp_shift`.
4. Execute prediction using 64-bit accumulation: `(qlp_coeffs[i] as i64) * (samples[n-1-i] as i64)`.

---

## 9. Rust-Specific Considerations

### 9.1 Wrapping Arithmetic and Overflow Safety
In Rust debug builds, integer overflow triggers a panic. In release builds, overflow wraps. To ensure absolute safety, Loom uses explicit primitive operations and debug assertions:

```rust
#[inline(always)]
pub fn predict_lpc_sample(
    history: &[i64],
    qlp_coeffs: &[i32],
    qlp_shift: i8,
    order: usize,
) -> i64 {
    debug_assert!(order <= history.len());
    debug_assert!(order <= qlp_coeffs.len());
    debug_assert!(qlp_shift >= 0);

    let mut acc: i64 = 0;
    for i in 0..order {
        // Safe 64-bit widening multiplication and addition
        acc += (qlp_coeffs[i] as i64) * history[order - 1 - i];
    }

    // Arithmetic right shift (floor division)
    acc >> qlp_shift
}
```

---

## 10. Benchmark Methodology

### 10.1 Evaluation Criteria
- **Bit-Exact Cross-Platform Verification:** Running round-trip encode/decode on x86-64 Linux, ARM64 macOS (M-series), and WebAssembly targets, verifying MD5 checksum equality ($MD5_{\text{decoded}} == MD5_{\text{original}}$).
- **Quantization Precision Optimization:** Measuring SNR and bit-cost across precision values $Q \in [5, 15]$.

---

## 11. References

1. **RFC 9639 (2024):** *FLAC Audio Coding Format.* Section 4.3: Prediction.
2. **Coalson, J. (2000):** *FLAC - Free Lossless Audio Codec Design.* Xiph.Org Foundation.
3. **Goldberg, D. (1991):** *What Every Computer Scientist Should Know About Floating-Point Arithmetic.* ACM Computing Surveys, Vol. 23, No. 1, pp. 5-48.

---

## 12. Open Research Questions

1. **Adaptive Per-Coefficient Precision:** Can individual coefficients within an LPC filter be stored at variable bit-precisions (e.g., $a_1$ at 14 bits, $a_{12}$ at 6 bits) to save header overhead without reducing residual variance reduction?

---

## 13. Future Improvements

- **Auto-Vectorized SIMD Accumulator (`vpmuldq`):** Implement 4-way 64-bit vector MAC loops using x86 AVX2 intrinsics for high-order LPC synthesis ($P = 32$).
