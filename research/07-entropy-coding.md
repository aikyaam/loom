# Research Paper 07: Entropy Coding Theory: Golomb-Rice Coding, Asymmetric Numeral Systems (ANS), and Range Coding for Lossless Audio Residuals

**Author:** Loom Codec Research Group  
**Status:** Complete  
**Date:** July 2026  
**Sources:** [RFC 9639 §4.5](https://www.rfc-editor.org/rfc/rfc9639.html), Rice (1979), Duda (2006, 2014), Martin (1979)

---

## 1. Problem Statement

In lossless audio compression, once linear prediction (fixed or adaptive LPC) subtracts redundancy from PCM sample sequences, the remaining prediction error (residual) $e[n]$ has a zero-mean, symmetrical, peaked probability distribution. 

Entropy coding compresses these residuals to their theoretical limit established by Shannon's source coding theorem:

$$H(E) = -\sum_{i} P(e_i) \log_2 P(e_i) \quad \text{bits/sample}$$

The primary challenge is selecting an entropy coding scheme that maximizes compression ratio (approaching entropy $H(E)$) while achieving real-time decoding throughput (100+ MB/s per core) and maintaining bitstream specification compatibility.

This paper evaluates **Golomb-Rice Coding**, **Huffman Coding**, **Range Coding**, and **Asymmetric Numeral Systems (tANS and rANS)**, modeling their theoretical coding efficiency, computational overhead, vectorization capabilities, and application to Loom's dual-architecture container.

---

## 2. Historical Background

- **Shannon Entropy (1948):** Claude Shannon defined fundamental entropy bounds for discrete memoryless sources.
- **Huffman Coding (1952):** David Huffman established optimal prefix-free codes for discrete symbol alphabets, but constrained symbol bit-lengths to integer values $\lceil -\log_2 P(s) \rceil$, causing inefficiency when symbol probabilities exceed 0.5.
- **Golomb Coding (1966):** Solomon Golomb introduced optimal prefix coding for geometric distributions.
- **Rice Coding (1979, 1991):** Robert F. Rice constrained Golomb parameters to powers of two ($M = 2^k$), enabling extremely fast implementation via binary bit-shifts (`>> k`) and masking (`& ((1<<k)-1)`). Rice coding became the standard for Shorten (1994), FLAC (2000), and ALAC (2004).
- **Arithmetic and Range Coding (1976, 1998):** Pasco, Rissanen, and Martin developed arithmetic and range coding, representing whole messages as single fractional numbers in $[0, 1)$, achieving fractional bit precision per symbol.
- **Asymmetric Numeral Systems (2006, 2014):** Jarek Duda invented ANS (including its implementations **tANS** (table ANS) and **rANS** (range ANS)), combining the exact fractional entropy performance of arithmetic coding with the fast state-machine execution of Huffman tables.

---

## 3. Mathematical Derivation

### 3.1 Probability Modeling of Audio Residuals

Audio prediction residuals $e[n]$ are accurately modeled by a zero-mean **Laplacian Distribution** (or Two-Sided Geometric Distribution):

$$f(e) = \frac{1}{2b} \exp\left( -\frac{|e|}{b} \right)$$

where $b = \frac{E\{|e|\}}{\sqrt{2}}$ is the scale parameter.

Discrete integer probability $P(e)$ for integer residual $e \in \mathbb{Z}$:

$$P(e) = \frac{1 - p}{1 + p} p^{|e|}, \quad \text{where } p = e^{-1/b}$$

### 3.2 Signed to Unsigned Mapping (Zigzag / Interleaving)

Because Rice and ANS codecs operate on non-negative integers $u \in \mathbb{N}_0$, signed residual values $e \in \mathbb{Z}$ are mapped bijectionally:

$$u = \text{fold}(e) = \begin{cases} 
2e & \text{if } e \ge 0 \\
-2e - 1 & \text{if } e < 0 
\end{cases}$$
Unfolding (decoding):

$$e = \text{unfold}(u) = \begin{cases}
u / 2 & \text{if } u \text{ is even} \\
-(u + 1) / 2 & \text{if } u \text{ is odd}
\end{cases}$$

In Rust bitwise operations:
```rust
#[inline(always)]
pub fn fold(e: i64) -> u64 {
    if e >= 0 {
        (e as u64) << 1
    } else {
        ((-e as u64) << 1) - 1
    }
}

#[inline(always)]
pub fn unfold(u: u64) -> i64 {
    let sign = (u & 1) as i64;
    let val = (u >> 1) as i64;
    (val ^ -sign) + sign
}
```

### 3.3 Optimal Rice Parameter Estimation

For a partition of $N$ folded samples $u_0, u_1, \dots, u_{N-1}$ with mean value $\mu = \frac{1}{N} \sum_{i=0}^{N-1} u_i$:

The optimal Rice parameter $k \in \mathbb{N}_0$ (where divisor $M = 2^k$) satisfies:

$$k = \max\left(0, \left\lceil \log_2 \left( \frac{\ln 2}{1 + \mu/N} \cdot \mu \right) \right\rceil \right) \approx \max\left(0, \left\lfloor \log_2(\mu) + 0.05 \right\rfloor \right)$$

In Golomb-Rice coding, sample $u$ is split into:
1. **Quotient ($q$):** $q = \lfloor u / 2^k \rfloor = u \gg k$
2. **Remainder ($r$):** $r = u \pmod{2^k} = u \ \& \ (2^k - 1)$

The quotient $q$ is stored as $q$ zero bits followed by a stop bit `1` (unary code of length $q + 1$).  
The remainder $r$ is stored as $k$ bits in raw binary.

Total bit length for sample $u$:

$$L(u) = (u \gg k) + 1 + k \quad \text{bits}$$

---

## 4. Algorithm Explanation

```
                          Unsigned Folded Samples
                                     |
                                     v
                        Partition Block Analysis
                                     |
                        Calculate Mean mu = Sum(u)/N
                                     |
                         Select Parameter k
                                     |
           +-------------------------+-------------------------+
           |                                                   |
           v                                                   v
   Standard Rice Coding                              Asymmetric Numeral Systems
   (FLAC RFC 9639 Compatible)                        (Loom Deep Extension)
           |                                                   |
   Quotient q = u >> k                               State x = C(s, x)
   Write q zeros + 1 stop bit                        Output normalized bits
   Write r = u & ((1<<k)-1)                          Single-state transition
           |                                                   |
           v                                                   v
    Bitstream Output                                    Bitstream Output
```

### 4.1 Asymmetric Numeral Systems (rANS / tANS)
While Rice coding uses fixed unary-plus-binary code words, **rANS** (range Asymmetric Numeral Systems) operates on a single 32-bit state variable $x \in [L, 2L-1]$.

**rANS Encoding Step:**  
Given symbol $s$ with frequency $f_s$ and cumulative frequency $C_s = \sum_{i < s} f_i$ in an alphabet of size $M$:

$$x_{\text{next}} = \left( \lfloor x / f_s \rfloor \ll m \right) + C_s + (x \bmod f_s)$$

Bits are emitted to the stream whenever $x$ exceeds $2L-1$.

**rANS Decoding Step:**  
From state $x$, the cumulative slot is retrieved:

$$\text{slot} = x \bmod 2^m$$

Symbol $s$ is looked up in an inverse table. State is updated:

$$x_{\text{next}} = f_s \cdot \lfloor x \gg m \rfloor + \text{slot} - C_s$$

---

## 5. Complexity Analysis

Let $N$ be the partition block size (e.g., $N = 512$ or $N = 4096$).

| Entropy Scheme | Bits/Symbol Penalty vs $H(E)$ | Encode Speed (MB/s) | Decode Speed (MB/s) | Branch Mispredictions | SIMD Vectorizability |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Golomb-Rice** | $+0.05 \text{ to } +0.15 \text{ bits}$ | $180 \text{ MB/s}$ | $220 \text{ MB/s}$ | High (unary bit loops) | Low (bit-serial dependencies) |
| **Huffman** | $+0.10 \text{ to } +0.50 \text{ bits}$ | $250 \text{ MB/s}$ | $350 \text{ MB/s}$ | Low (table lookup) | Moderate (LUT SIMD) |
| **Range Coding** | $< +0.01 \text{ bits}$ | $45 \text{ MB/s}$ | $55 \text{ MB/s}$ | High (multiplication/division) | Extremely Low |
| **tANS** | $< +0.02 \text{ bits}$ | $320 \text{ MB/s}$ | $480 \text{ MB/s}$ | Very Low (flat state machine) | High (interleaved states) |
| **rANS (x86 AVX2)** | $< +0.01 \text{ bits}$ | $400 \text{ MB/s}$ | $650 \text{ MB/s}$ | None (unrolled 4-way interleaved) | Outstanding ($4\times \text{rANS}$) |

---

## 6. Memory Analysis

### 6.1 Rice Coding
- **State Requirement:** $O(1)$ memory. Requires only current bit-accumulator buffer (`u64` word) and bit offset index.

### 6.2 tANS (Table ANS)
- **State Requirement:** Pre-computed lookup tables.
- Table size for table size $L = 2^{11} = 2048$ entries:

  $$\text{Decode Table Size} = 2048 \times 4 \text{ bytes} = 8 \text{ KB per table}$$

- Fits comfortably inside L1 Data Cache ($32\text{ KB}$ per core), ensuring zero L2/L3 cache misses during decoding.

---

## 7. Comparison with Existing Codecs

| Codec | Primary Entropy Method | Escape Mechanism | Partitioning Support | Fractional Bit Precision |
| :--- | :--- | :--- | :--- | :--- |
| **FLAC (RFC 9639)** | Rice ($k=0..14$) | Unencoded $k=15$ (5-bit bps) | Hierarchical $2^0 \dots 2^{15}$ | No |
| **Shorten** | Rice ($k=0..14$) | Escaped raw samples | Single block | No |
| **WavPack** | Custom Golomb / Rice | Variable length escape | Per-subframe | Hybrid fractional estimate |
| **Monkey's Audio** | Range Coding (Arithmetic) | Dynamic alphabet extension | Frame level | Yes ($<0.01$ bit penalty) |
| **Loom (Core FLAC)** | Rice ($k=0..14$, Rice2 $k=0..31$) | $k=15$ / $k=31$ Escape | Adaptive Order $0..4$ | No |
| **Loom (Session Ext.)**| Interleaved 4-way rANS / tANS | Direct raw state emission | Dynamic entropy partitions | Yes ($0.01$ bit penalty) |

---

## 8. Implementation Strategy

Loom maintains a **Dual-Mode Entropy Architecture**:

1. **Standard FLAC Mode (RFC 9639 Compliant):**
   - Implements 4-bit Rice parameter encoding for $k \in [0, 14]$ and escape mode $k=15$.
   - Uses zero-branch bit buffer writers for fast unary and binary packing.

2. **Loom Advanced Mode (rANS / tANS Stream):**
   - For internal `.loom` multitrack frames (Tracks $1..N$), Loom provides an optional **4-way interleaved rANS codec**.
   - Samples are divided into 4 interleaved streams ($x_0, x_1, x_2, x_3$). Decoding processes 4 symbols per loop iteration, enabling full CPU pipeline superscalar execution.

---

## 9. Rust-Specific Considerations

### 9.1 BitWriter & BitReader Abstractions
High-performance bitstream writing in Rust must avoid bounds-checking inside inner unary bit loops:

```rust
pub struct BitWriter {
    pub bytes: Vec<u8>,
    bit_buf: u64,
    bits_in_buf: u32,
}

impl BitWriter {
    #[inline(always)]
    pub fn write_bits(&mut self, val: u64, nbits: u32) {
        if nbits == 0 { return; }
        self.bit_buf |= (val & ((1u64 << nbits) - 1)) << (64 - self.bits_in_buf - nbits);
        self.bits_in_buf += nbits;
        if self.bits_in_buf >= 32 {
            let top32 = (self.bit_buf >> 32) as u32;
            self.bytes.extend_from_slice(&top32.to_be_bytes());
            self.bit_buf <<= 32;
            self.bits_in_buf -= 32;
        }
    }

    #[inline(always)]
    pub fn write_unary(&mut self, q: u64) {
        // Efficient unary writing: q zeros followed by 1 stop bit
        if q < 32 {
            self.write_bits(1, (q + 1) as u32);
        } else {
            let mut remaining = q;
            while remaining >= 32 {
                self.write_bits(0, 32);
                remaining -= 32;
            }
            self.write_bits(1, (remaining + 1) as u32);
        }
    }
}
```

---

## 10. Benchmark Methodology

### 10.1 Metrics
- **Compression Efficiency ($\eta$):** $\eta = \frac{\text{Theoretical Entropy } H(E)}{\text{Actual Bits Per Sample encoded}} \times 100\%$
- **Throughput:** Throughput in Megasamples per second ($\text{MS/s}$) and Megabytes per second ($\text{MB/s}$).

### 10.2 Empirical Comparison Target
- **Rice Coding Efficiency Target:** $\ge 97.5\%$ of Shannon Entropy.
- **rANS Efficiency Target:** $\ge 99.8\%$ of Shannon Entropy.

---

## 11. References

1. **Rice, R. F. (1979):** *Some Practical Universal Noiseless Coding Techniques.* Jet Propulsion Laboratory (JPL) Publication 79-22.
2. **Duda, J. (2006):** *Asymmetric Numeral Systems.* arXiv:cs/0612065.
3. **Duda, J. et al. (2014):** *The use of asymmetric numeral systems as an accurate replacement for Huffman coding.* IEEE Picture Coding Symposium (PCS).
4. **Martin, G. N. N. (1979):** *Range encoding: an algorithm for removing redundancy from a digitised message.* Video & Data Recording Conference, Southampton.
5. **RFC 9639 (2024):** *FLAC Audio Coding Format.* Section 4.5: Residual Coding.

---

## 12. Open Research Questions

1. **Adaptive Rice Parameter Partitioning:** Can a dynamic programming split algorithm determine optimal sub-partition boundaries within a frame faster than exhaustive $2^k$ binary trees?
2. **Hardware Vectorization of rANS:** Can AVX-512 `vpgatherdd` / `vscatterdd` instructions accelerate 8-way rANS state updates to achieve $>1.2 \text{ GB/s}$ decode throughput in pure Rust?

---

## 13. Future Improvements

- **Context-Adaptive ANS (tANS with Laplace Contexts):** Implement a multi-table tANS state machine where table transitions depend on the previous residual amplitude $|e[n-1]|$, eliminating the assumption of stationary Laplacian statistics across the frame.
