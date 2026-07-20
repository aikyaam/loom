# Research Paper 04: Stereo Decorrelation

**Author:** Loom Codec Research Group  
**Status:** Complete  
**Date:** July 2026  

**Sources**: [RFC 9639 §4.2](https://www.rfc-editor.org/rfc/rfc9639.html#name-interchannel-decorrelation), [Old FLAC Format](https://xiph.org/flac/old_format.html#interchannel)

---

## Interchannel Correlation in Stereo Audio

Stereo audio files (2 channels) usually display high correlation between the left and right channels, as audio is often panned or shares central content. Coding channels independently wastes space. Decorrelation maps left/right channels to alternate channels, reducing overall variance.

FLAC defines four stereo channel assignment modes, chosen per frame:

1. **Independent (0-7)**: Left and right channels are coded independently.
2. **Left/Side (8)**: Left channel and Side channel are coded.
3. **Right/Side (9)**: Right channel and Side channel are coded.
4. **Mid/Side (10)**: Mid channel and Side channel are coded.

---

## Channel Transformation Math

Let $L[n]$ be the Left channel sample, and $R[n]$ be the Right channel sample at sample index $n$.

### 1. Mid/Side (MS)
$$\text{Mid}[n] = \lfloor \frac{L[n] + R[n]}{2} \rfloor$$
$$\text{Side}[n] = L[n] - R[n]$$

**Reconstruction**:
$$L[n] = \text{Mid}[n] + \lfloor \frac{\text{Side}[n] + 1}{2} \rfloor$$
$$R[n] = \text{Mid}[n] - \lfloor \frac{\text{Side}[n]}{2} \rfloor$$

*Proof of exact reconstruction:*
Let's verify:
If we substitute $\text{Mid}$ and $\text{Side}$:
$$\text{Side} \text{ is odd} \implies \text{Side} = 2k+1 \implies L - R = 2k+1 \implies L + R = 2\text{Mid} + 1$$
If $L - R$ is odd, $L + R$ is also odd (since $L-R$ and $L+R$ have the same parity).
$$\text{Mid} = \lfloor \frac{L+R}{2} \rfloor = \frac{L+R-1}{2}$$
$$L = \text{Mid} + \lfloor \frac{2k+1+1}{2} \rfloor = \text{Mid} + k + 1 = \frac{L+R-1}{2} + \frac{L-R-1}{2} + 1 = L$$
$$R = \text{Mid} - \lfloor \frac{2k+1}{2} \rfloor = \text{Mid} - k = \frac{L+R-1}{2} - \frac{L-R-1}{2} = R$$
Works exactly, preventing roundoff error!

### 2. Left/Side (LS)
$$\text{Left}[n] = L[n]$$
$$\text{Side}[n] = L[n] - R[n]$$

**Reconstruction**:
$$R[n] = L[n] - \text{Side}[n]$$

### 3. Right/Side (RS)
$$\text{Right}[n] = R[n]$$
$$\text{Side}[n] = L[n] - R[n]$$

**Reconstruction**:
$$L[n] = R[n] + \text{Side}[n]$$

---

## Word Size Expansion

Decorrelation can increase the dynamic range of the side channel:
- $\text{Side} = L - R$ can require up to 1 extra bit of precision.
- E.g., if $L, R$ are 16-bit signed, Side can range from $-65535$ to $+65535$, which requires 17 bits.
- Therefore, the subframe representing the Side channel must be coded with $b + 1$ bits per sample (where $b$ is the original sample bit depth).
- Left-side and Right-side only expand the Side channel (so Left remains $b$ bits, Side is $b+1$ bits).
- Mid-side encodes Mid (which is $b$ bits) and Side (which is $b+1$ bits).

---

## Encoder Decision Rule

The encoder evaluates all 4 stereo modes for every block by checking the sum of absolute values or estimated entropy of the decorrelated samples. The mode with the smallest total estimated size is selected.
Estimate formula for channel $c$:
$$\text{cost}(c) = \sum_n |x_c[n]|$$
Total cost is $\text{cost}(ch1) + \text{cost}(ch2)$ adjusted for bit-depth expansion.

---

## Loom Implementation

Loom will adopt these stereo decorrelation methods for 2-channel sessions. We will represent the chosen stereo mode in the frame header of Loom's bitstream, just like FLAC.
All decorrelation transforms will use `i64` math to prevent overflow.
```rust
pub enum StereoMode {
    Independent,
    LeftRight, // same as independent
    LeftSide,
    RightSide,
    MidSide,
}
```
Each block has a `StereoMode`. The decoder reads the mode and applies the reconstruction formulas.

---

## References

1. **RFC 9639 (2024):** *FLAC Audio Coding Format.* Section 4.2: Interchannel Decorrelation. [https://www.rfc-editor.org/rfc/rfc9639.html#name-interchannel-decorrelation](https://www.rfc-editor.org/rfc/rfc9639.html#name-interchannel-decorrelation)
2. **Blumlein, A. D. (1933):** *Improvements in and relating to Sound-transmission, Sound-recording and Sound-reproducing Systems.* UK Patent 394,325.
