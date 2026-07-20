# Research Paper 08: Version Diffing

**Author:** Loom Codec Research Group  
**Status:** Complete  
**Date:** July 2026  

**Sources**: RFC 3284 (VCDIFF) specification, `bsdiff` algorithm, git packfile delta compression.

---

## Concept

In a DAW production environment, users export a session many times, making small edits or minor level adjustments. Re-saving the entire session takes up massive space.
Loom version diffing stores a new session export as a delta against a base version.

---

## Alternative Approaches

### 1. Compressed Bitstream-Level Diff (VCDIFF on raw `.loom` bytes)
We could run a standard binary delta (like VCDIFF) on `v1.loom` and `v2.loom`.
*Pros*: Simple, uses standard RFC 3284 tools.
*Cons*: Fragile. A tiny change in an audio sample (e.g. gain change or minor shift) cascades through the prediction step and entropy coder (Rice coding), producing completely different bit patterns downstream. The bitstream diff will be almost as large as a full encode.

### 2. Pre-Entropy Residual-Level Diff
Diff the prediction residuals or audio samples *before* Rice coding.
*Pros*: If a level changes by +1dB, the residuals are highly similar, only scaled. If a section is muted, the residuals become zero.
*Cons*: Requires a custom diff implementation integrated into the codec.

### 3. Block-Level Frame Re-use (Deduplication)
Since Loom is block-based (e.g., blocks of 4096 samples), if a track has no edits in a 4096-sample region, its compressed frame in version 2 will be identical to version 1.
We can diff at the frame/block level:
- Compute MD5 hashes of raw PCM blocks or compressed frames in `v1`.
- In `v2`, if a frame's content is identical, instead of storing the compressed frame, store a reference: `Ref(v1_frame_index)`.
- If a frame is modified, store the new compressed frame.
- If a frame has minor edits (e.g., gain change), we can store either the new frame or a simple residual delta.

---

## Loom's Selected Design: Frame-Level Deduplication and Delta Coding

Loom will use **Frame-Level Delta & Deduplication** (similar to Git packfile deltas):
1. For each track, version 2 contains a list of frames.
2. For each frame in version 2, the encoder searches version 1 for a matching frame:
   - If a frame has the same PCM MD5 hash, serialize it as a `CopyFrame(v1_frame_index)`.
   - If no exact match is found, serialize it as a `NewFrame(compressed_data)`.
3. This is robust to edits: if a user edits 1 second of a 5-minute song, only the frames covering that 1 second are stored as `NewFrame`s. The remaining 299 seconds are stored as references (`CopyFrame`), resulting in a tiny `.diff` file.

---

## Bitstream Diff Representation

A `.diff` file or block will be represented as:
- Base file reference (e.g., MD5 hash of `v1.loom`).
- Metadata updates (new track configurations, new seek indices).
- For each track, a stream of frame instructions:
  - `0x00`: Copy frame from base (stores 32-bit frame index).
  - `0x01`: Insert new frame (stores length prefix + raw frame bytes).

### Reconstruction
`apply-diff v1.loom diff -> v2.loom`:
1. Open `v1.loom` and read its frame index.
2. Read the diff instructions.
This achieves byte-exact reconstruction of `v2.loom`.

---

## References

1. **Tridgell, A. (1999):** *Efficient Algorithms for Sorting and Synchronization.* PhD thesis, Australian National University.
2. **MacDonald, J. (2000):** *File Difference Embedding Models on Information Topology.* Master's thesis, UC Berkeley.
3. **Percival, C. (2003):** *Naive Difference Algorithms for Executables.* BSDiff Documentation.
