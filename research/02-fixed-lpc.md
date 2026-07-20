# Research Paper 02: Fixed Linear Prediction

**Author:** Loom Codec Research Group  
**Status:** Complete  
**Date:** July 2026  

**Sources**: [RFC 9639 §4.3](https://www.rfc-editor.org/rfc/rfc9639.html#name-prediction), [Old FLAC Format](https://xiph.org/flac/old_format.html#prediction), Shorten paper (Robinson, 1994)

---

## Concept

Linear prediction models a signal sample as a linear combination of previous samples:

```
x̂[n] = sum_{i=1}^{p} a_i * x[n-i]
```

The residual (prediction error) is: `e[n] = x[n] - x̂[n]`

For a good predictor, residuals have much smaller variance than the original signal → fewer bits needed.

---

## FLAC Fixed Predictors (Orders 0–4)

FLAC defines 5 fixed predictors with hardcoded integer coefficients. They are efficient (no per-block coefficient storage) but less accurate than adaptive LPC.

Derived from finite differences. For a smooth signal, differences trend toward zero.

| Order | Prediction formula | Warm-up samples |
|-------|--------------------|-----------------|
| 0 | `x̂[n] = 0` | 0 |
| 1 | `x̂[n] = x[n-1]` | 1 |
| 2 | `x̂[n] = 2*x[n-1] - x[n-2]` | 2 |
| 3 | `x̂[n] = 3*x[n-1] - 3*x[n-2] + x[n-3]` | 3 |
| 4 | `x̂[n] = 4*x[n-1] - 6*x[n-2] + 4*x[n-3] - x[n-4]` | 4 |

These correspond exactly to the coefficients of Pascal's triangle with alternating signs (binomial coefficients for finite differences), derived as:
- Order k predictor: `Δ^k x[n] = 0` (k-th finite difference is constant)
- The warmup samples are stored verbatim (bps bits each, big-endian signed integer)

**Residual computation** (encoder):
```
for n in order..block_size:
    residual[n] = x[n] - predict(x, n, order)
```

**Reconstruction** (decoder):
```
for n in order..block_size:
    x[n] = residual[n] + predict(x, n, order)
```

Since reconstruction is sequential and x[0..order-1] are stored verbatim, each x[n] is fully recoverable.

---

## Encoder: Choosing the Best Fixed Order

The encoder tries all orders 0–4 and picks the one producing the smallest encoded output (residuals + overhead). The rough heuristic is to estimate bits needed based on the sum of |residuals|:

```
estimated_bits ≈ n * (log2(2 * mean(|residuals|) + 1) + 1)
```

This is cheap to compute without actually encoding to bits.

---

## Implementation Notes for Loom

1. The warm-up samples must be stored before the residual in each subframe.
2. The residual (after warm-up samples are subtracted out) is Rice-coded.
3. For order 0: all samples are residuals. For silence (all zeros), the CONSTANT subframe is more efficient.
4. Arithmetic must be done in signed 64-bit to avoid overflow for 24-bit audio with high-order predictors.

### Rust implementation approach

```rust
fn predict_fixed(samples: &[i64], n: usize, order: usize) -> i64 {
    match order {
        0 => 0,
        1 => samples[n-1],
        2 => 2*samples[n-1] - samples[n-2],
        3 => 3*samples[n-1] - 3*samples[n-2] + samples[n-3],
        4 => 4*samples[n-1] - 6*samples[n-2] + 4*samples[n-3] - samples[n-4],
        _ => unreachable!(),
    }
}
```

---

## References

1. **RFC 9639 (2024):** *FLAC Audio Coding Format.* Section 4.3.2: Fixed Predictors. [https://www.rfc-editor.org/rfc/rfc9639.html#name-fixed-prediction](https://www.rfc-editor.org/rfc/rfc9639.html#name-fixed-prediction)
2. **Robinson, T. (1994):** *SHORTEN: Simple Lossless and Near-Lossless Audio Compression.* Technical Report CUED/F-INFENG/TR.156, Cambridge University Engineering Department.
