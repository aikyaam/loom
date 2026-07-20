# Research Paper 05: Cross-Track Prediction

**Author:** Loom Codec Research Group  
**Status:** Complete  
**Date:** July 2026  

**Sources**: WavPack multichannel decorrelation documentation, FLAC interchannel decorrelation concepts.

---

## Concept: Generalizing Decorrelation

WavPack and other multi-channel/session audio formats achieve high compression by exploiting correlation across channels.
For a multi-track recording session (e.g., drum stems, vocals, guitar tracks), there is significant redundancy:
1. **Shared Silences**: Different instruments are silent at the same time or have bleed from other mics.
2. **Shared Transients & Tempo**: Multi-track sessions share timing and frequency characteristics.

Loom extends stereo decorrelation to cross-track prediction. Instead of just L/R decorrelation, we allow track $B$'s residual to be panned or predicted from track $A$'s residual.

---

## Mathematical Formulation

Let $E_A[n]$ be the residual of track $A$ after per-track prediction (LPC or Fixed).
Let $E_B[n]$ be the residual of track $B$ after per-track prediction.

We can model the correlation between $E_A$ and $E_B$ using a first-order cross-track predictor:

$$\hat{E}_B[n] = w \cdot E_A[n]$$

where $w$ is a quantized coupling weight. The new cross-track residual for $B$ is:

$$e_B[n] = E_B[n] - \text{round}(w \cdot E_A[n])$$

If $w$ is chosen correctly, the variance of $e_B$ is much smaller than $E_B$.

### Weight Estimation
The optimal weight $w$ can be computed using least-squares correlation:

$$w = \frac{\sum_n E_A[n] \cdot E_B[n]}{\sum_n E_A[n]^2}$$

We quantize $w$ to a fixed-point integer (e.g., $Q4.12$ or simple 8-bit quantization) so that the decoder can reconstruct the signal exactly using integer arithmetic. Let $W_q$ be the integer weight:

$$W_q = \text{round}(w \cdot 256) \implies w \approx \frac{W_q}{256}$$

$$\hat{E}_B[n] = (W_q \cdot E_A[n]) \gg 8$$

$$e_B[n] = E_B[n] - \hat{E}_B[n]$$

### Reconstruction

$$E_B[n] = e_B[n] + ((W_q \cdot E_A[n]) \gg 8)$$

---

## Dependency & Independent Decoding Constraint

> [!IMPORTANT]
> The build plan states: "encoding 'predict this track's residual partly from another track's residual' in the bitstream without breaking independent per-track decodability when only one track is needed."

If track $B$ depends on track $A$, to decode a time-range in track $B$, we must also decode track $A$ for that frame.
To make this practical:
1. Limit dependency chains: track $B$ can only depend on one other track $A$, and track $A$ itself must be independently coded (no circular or deep transitive dependencies).
2. The seek index and headers must explicitly flag which track is the parent/reference track.
3. If a user decodes *only* track $B$, our decoder library will automatically and transparently decode the reference frames of track $A$ under the hood, apply cross-prediction, and discard track $A$'s samples.

---

## Decision Rule for Cross-Track Prediction

The encoder should only apply cross-track prediction if the size reduction outweighs the overhead:
1. Compute the per-track residuals $E_A$ and $E_B$.
2. Compute the cross-correlation weight $W_q$ of $E_B$ relative to $E_A$.
3. Compute $e_B[n] = E_B[n] - ((W_q \cdot E_A[n]) \gg 8)$.
5. If $\text{cost}_{\text{cross}} < \text{cost}_{\text{indep}}$, enable cross-prediction, serialize $W_q$ in track $B$'s frame header, and record $A$ as the reference. Otherwise, save track $B$ independently.

---

## References

1. **Openshaw, D. (2002):** *WavPack Transitional Lossless Audio Compression.* WavPack Documentation. [https://www.wavpack.com](https://www.wavpack.com)
2. **Den Brinker, A. C. et al. (2009):** *Joint Channel Coding in Lossless Audio Compression.* IEEE Transactions on Audio, Speech, and Language Processing.
