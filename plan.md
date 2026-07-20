# Loom Roadmap: Research-Driven Lossless Audio Codec + Session Container

## Vision

Loom is **not just another audio container** and **not just another FLAC encoder**.

Loom has two complementary goals:

1. **A research-driven, high-performance lossless audio encoder** capable of producing standard FLAC-compatible output through improved prediction algorithms and encoding strategies.
2. **A session-aware multitrack container** that extends lossless compression beyond individual tracks by exploiting correlation between stems while supporting non-destructive editing, fast seeking, and efficient versioning.

These two goals share the same compression engine.

---

# Core Architecture

```
                    Loom
                      │
      ┌───────────────┴────────────────┐
      │                                │
  Codec Engine                  Session Container
      │                                │
Prediction                   Track Metadata
Entropy Coding               Edit Overlays
Frame Encoding               Seek Tables
FLAC Output                  Version Diffing
```

The codec layer is completely independent from the session layer.

The session container builds on top of the codec.

---

# Development Phases

## Phase 1: Core Codec

Goal:

Create a modern lossless encoder capable of producing standard FLAC-compatible output.

Research areas:

* FLAC bitstream specification
* Fixed Predictors
* Adaptive LPC
* Levinson-Durbin recursion
* Burg LPC
* Rice Coding
* Stereo decorrelation
* Block size selection
* Predictor selection
* SIMD optimization
* Parallel encoding

CLI:

```
loom encode input.wav output.flac
```

This command should produce a completely standard FLAC file.

Success criteria:

* Valid FLAC output
* Comparable decoding compatibility
* Better compression or better encoding strategies than existing encoders
* Extensive benchmark suite

---

## Phase 2: Codec Research Platform

Goal:

Turn Loom into a research platform for experimenting with prediction algorithms.

Commands:

```
loom benchmark
loom analyze
loom compare
```

Examples:

```
loom benchmark samples/
```

Outputs:

* Compression ratio
* Encoding speed
* Decoding speed
* CPU usage
* Memory usage

```
loom analyze song.wav
```

Outputs:

* Predictor chosen
* LPC order
* Rice parameters
* Residual entropy
* Block size analysis

This phase should contain all benchmarking infrastructure.

---

## Phase 3: Session Container

Goal:

Create a multitrack session container powered by the Loom codec.

Features:

* Multiple tracks
* Track metadata
* Session metadata
* Album art
* Seek tables
* Track indexing

Commands:

```
loom encode-session stems/ session.loom
```

```
loom decode-session session.loom output/
```

The container should reuse the codec implementation from Phase 1.

---

## Phase 4: Cross-Track Compression

This is the primary research contribution.

Research topics:

* Cross-track correlation
* Automatic reference track selection
* Correlation graph construction
* Predictor weight estimation
* Adaptive coupling
* Entropy reduction measurements

Pipeline:

```
Track Analysis

↓

Correlation Matrix

↓

Reference Track Selection

↓

Cross-Track Prediction

↓

Residual Coding

↓

Frame Encoding
```

This phase should be heavily benchmarked.

Compare:

* Individual FLAC files
* Loom session compression

Measure:

* Compression improvement
* CPU usage
* Decode performance

---

## Phase 5: Non-Destructive Editing

Features:

* Gain automation
* Fade-in
* Fade-out
* Mute regions
* Clip offsets
* Edit metadata

Commands:

```
loom edit
```

```
loom render
```

Edits must modify metadata only.

Audio should never be recompressed.

---

## Phase 6: Fast Seeking

Implement:

* Frame index
* Time index
* Track index
* O(1) seeking
* Partial decoding
* Region extraction

Commands:

```
loom extract
```

```
loom seek
```

---

## Phase 7: Versioning

Implement:

* Frame hashes
* Frame comparison
* Delta generation
* Delta application

Commands:

```
loom diff
```

```
loom apply-diff
```

The goal is efficient storage of multiple project revisions.

---

# CLI Design

## Standard FLAC Encoding

```
loom encode input.wav output.flac
```

Produces:

* Standard FLAC
* Playable everywhere

---

## Session Encoding

```
loom encode-session stems/ session.loom
```

Produces:

* Loom session
* Multiple tracks
* Cross-track compression
* Metadata
* Edit overlays
* Version support

---

## Analysis

```
loom analyze input.wav
```

---

## Benchmark

```
loom benchmark samples/
```

---

## Session Editing

```
loom edit
```

---

## Rendering

```
loom render
```

---

## Diff

```
loom diff
```

---

## Apply Diff

```
loom apply-diff
```

---

# Repository Structure

```
loom/
├── loom-core/
│   ├── prediction/
│   ├── lpc/
│   ├── fixed/
│   ├── rice/
│   ├── stereo/
│   ├── encoder/
│   ├── decoder/
│   └── benchmark/
│
├── loom-session/
│   ├── container/
│   ├── metadata/
│   ├── edits/
│   ├── seek/
│   ├── diff/
│   └── render/
│
├── loom-cli/
│
├── research/
│   ├── 01-flac-format.md
│   ├── 02-fixed-lpc.md
│   ├── 03-adaptive-lpc.md
│   ├── 04-burg.md
│   ├── 05-rice.md
│   ├── 06-block-size.md
│   ├── 07-cross-track.md
│   ├── 08-seek.md
│   ├── 09-diff.md
│   ├── 10-benchmarks.md
│   └── 11-simd.md
│
└── benchmarks/
```

---

# Long-Term Objective

Loom should become:

* A research implementation of modern lossless audio compression.
* A high-quality FLAC-compatible encoder.
* A multitrack session container for professional audio workflows.
* A benchmark suite for evaluating prediction and entropy coding techniques.
* A reference implementation for future research into cross-track lossless compression.

The guiding principle is to separate the **compression engine** from the **session container**, allowing each to evolve independently while sharing the same underlying codec technology.
