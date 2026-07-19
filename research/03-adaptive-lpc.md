# Research Note 03: Adaptive LPC via Levinson-Durbin

**Sources**: [RFC 9639 §4.3](https://www.rfc-editor.org/rfc/rfc9639.html#name-prediction), [Old FLAC Format](https://xiph.org/flac/old_format.html#prediction), Levinson (1947), Durbin (1960)

---

## Motivation

Fixed predictors use preset coefficients. Adaptive LPC computes per-block optimal coefficients from the block's own autocorrelation, achieving better compression — especially for tonal/periodic audio like music.

---

## Step 1: Windowing (Optional but used by FLAC reference encoder)

Apply a Hann (or Welch/Bartlett) window to the block before computing autocorrelation. This reduces spectral leakage and produces better LPC coefficients.

```
windowed[i] = sample[i] * 0.5 * (1 - cos(2π*i / (N-1)))  // Hann window
```

FLAC reference encoder uses a Hann window; Loom will do the same.

---

## Step 2: Autocorrelation

Compute autocorrelation coefficients `R[0..order]`:

```
R[k] = sum_{n=0}^{N-1-k} windowed[n] * windowed[n+k]
```

If `R[0] == 0`, the block is silence — use CONSTANT or VERBATIM subframe.

---

## Step 3: Levinson-Durbin Recursion

Computes exact LPC coefficients from autocorrelation coefficients in O(order²) time.

```
// Initialize
a[1] = -R[1] / R[0]
E = R[0] * (1 - a[1]^2)

// Recurse for orders 2..p
for i in 2..=p:
    lambda = -R[i] - sum_{j=1}^{i-1} a[j] * R[i-j]
    k_i = lambda / E
    // Update coefficients (using temp array to avoid overwriting)
    for j in 1..i:
        a_new[j] = a[j] + k_i * a[i-j]
    a[i] = k_i
    a_new[i] = k_i
    a = a_new
    E = E * (1 - k_i^2)
```

The reflection coefficients k_i have |k_i| < 1 for a valid (stable) predictor. If |k_i| ≥ 1, the model is unstable and we fall back to fixed predictors.

The prediction formula: `x̂[n] = -sum_{i=1}^{p} a[i] * x[n-i]`

---

## Step 4: Coefficient Quantization

FLAC uses integer arithmetic throughout. The floating-point LPC coefficients must be quantized:

```
// Choose precision bits (qlp_coeff_precision), typically 12–15 bits
// Find the maximum absolute coefficient
max_coeff = max(|a[i]|)
// Shift so that max_coeff * 2^precision < 2^(qlp_coeff_bits-1)
qlp_shift = ceil(log2(max_coeff)) + precision - qlp_coeff_bits + 1
// Quantize
qlp_coeff[i] = round(a[i] * 2^precision)
```

The shift value and each quantized coefficient are stored in the bitstream.

**Reconstruction** with integer coefficients:
```
x[n] = residual[n] + (sum_{i=1}^{p} qlp_coeff[i] * x[n-i]) >> qlp_shift
```

All arithmetic in 64-bit signed integers.

---

## Step 5: Estimating Encoded Size

After computing residuals with quantized LPC, estimate Rice-coded bits:
```
est_bits ≈ N * (log2(2 * mean(|residuals|) + 1) + 1)
         + order * qlp_precision  // coefficient storage overhead
```

Compare vs fixed predictor estimate; use whichever is smaller.

---

## FLAC LPC Subframe Bitstream Layout

```
SUBFRAME_LPC:
  warm-up samples: order × bps bits each (verbatim, signed)
  qlp_coeff_precision: 4 bits (stored as value-1, so 0=1-bit precision)
  qlp_shift: 5 bits (signed)
  qlp_coefficients: order × qlp_coeff_precision bits each (signed)
  RESIDUAL (Rice-coded)
```

---

## Implementation Notes for Loom

1. Autocorrelation + windowing: use f64 arithmetic.
2. Levinson-Durbin: use f64, check stability (|k_i| ≥ 1 → fallback).
3. Quantization: convert to i64 coefficients and store precision + shift.
4. Decoder: pure integer arithmetic — no floats needed at decode time.
5. Order search: try orders 2, 4, 6, 8, 10, 12 (even orders tend to be better for audio) and the fixed predictors; pick smallest estimated bitcount.
