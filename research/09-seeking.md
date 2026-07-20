# Research Paper 07: Time-Range Random Access

**Author:** Loom Codec Research Group  
**Status:** Complete  
**Date:** July 2026  

**Sources**: FLAC SEEKTABLE metadata spec, Matroska (MKV) cue points indexing.

---

## Seek Indexing Concept

To support fast seeking without scanning the entire bitstream from the start, a seek table is necessary.
A seek table contains a series of "seek points". In Loom, each seek point maps a specific audio sample offset (time) to the exact byte offset of the frame containing that sample.

---

## Loom Seek Index Design

Loom's session container will contain a seek table metadata block at the beginning of the file, containing index tables for each track.

### Seek Point Schema
```rust
pub struct SeekPoint {
    pub sample_number: u64, // Sample index (time offset)
    pub byte_offset: u64,   // Byte offset from start of first audio frame
    pub frame_samples: u32, // Number of samples in the frame
}
```

To optimize seeking:
- Seek points are sorted in ascending order of `sample_number`.
- Seek points are placed at regular intervals (e.g., every 1 second or 44,100 samples).

---

## API for Random Access: `decode_range`

To decode a time range `decode_range(track, start_sample, end_sample)`:
1. Consult the seek table for the target `track`.
2. Perform a binary search on the seek points to find the last seek point where `sample_number <= start_sample`.
3. If no seek point matches, start from the first frame.
4. Jump the stream reader to the `byte_offset` corresponding to that seek point.
5. Decode frames sequentially until `end_sample` is reached.
6. Discard samples at the beginning of the first decoded frame that occur before `start_sample`.
7. Truncate samples at the end of the last decoded frame that occur after `end_sample`.

---

## Independent Seek Performance: $O(1)$ Time Complexity

Without a seek index, finding a range in a variable-bitrate stream requires reading and parsing every frame from the beginning of the file, which is an $O(N)$ operation (where $N$ is file length).
With a seek index:
1. Binary search the seek table: $O(\log S)$ where $S$ is the number of seek points. Since $S \ll N$ and the table fits in RAM, this is effectively instantaneous.
Thus, extraction time is independent of session length, providing true $O(1)$ seek overhead relative to session length.

---

## References

1. **RFC 9639 (2024):** *FLAC Audio Coding Format.* Section 4.4: Seektable. [https://www.rfc-editor.org/rfc/rfc9639.html#name-seektable](https://www.rfc-editor.org/rfc/rfc9639.html#name-seektable)
2. **Cormen, T. H. et al. (2009):** *Introduction to Algorithms.* MIT Press. Section 12: Binary Search Trees.
