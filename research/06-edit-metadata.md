# Research Note 06: Edit Metadata Layer

**Sources**: Apple/Pixar OpenTimelineIO (OTIO) specification, Audacity AUP3 project format.

---

## Non-Destructive Editing

Traditional audio compression applies edits (fades, gain automation, mutes) by rendering the audio to PCM and re-encoding. This is slow and introduces re-compression losses (for lossy formats) or takes up unnecessary space.
Loom stores non-destructive editing instructions in a separate metadata block in the container. The compressed audio frames remain unchanged. During decoding, these edit instructions are applied on-the-fly to the reconstructed PCM.

---

## Edit Instruction Schema

We define a JSON-like or binary-serialized metadata structure representing edit instructions on a per-track basis.

### 1. Mute Regions
Mutes are defined as time intervals in samples or milliseconds relative to the timeline start.
```rust
pub struct MuteRegion {
    pub start_sample: u64,
    pub end_sample: u64,
}
```

### 2. Fade In / Fade Out
Fades define a volume transition over a time range. Fades have shapes:
- **Linear**: $g(t) = \frac{t - t_0}{t_1 - t_0}$
- **Exponential**: $g(t) = 10^{2 \cdot (t - t_1)/(t_1 - t_0)}$ (or similar log-based)
- **S-Curve (Cosine)**: $g(t) = 0.5 \cdot (1 - \cos(\pi \cdot \frac{t - t_0}{t_1 - t_0}))$

```rust
pub enum FadeShape {
    Linear,
    Exponential,
    SCurve,
}

pub struct Fade {
    pub start_sample: u64,
    pub end_sample: u64,
    pub shape: FadeShape,
    pub is_fade_in: bool,
}
```

### 3. Gain Envelope (Automation)
A list of control points $(t, \text{gain})$, where $\text{gain}$ is a multiplier (float). Gain values between points are linearly interpolated.
```rust
pub struct GainPoint {
    pub sample_offset: u64,
    pub gain: f32, // multiplier, e.g. 1.0 = unity, 0.0 = silence
}
```

---

## Metadata Block Serialization

In the Loom bitstream, the edit metadata block is stored as a special metadata block type:
- Length: 24-bit integer
- Payload: Serialization of the `EditBlock` containing the above structs for each track.
- Editing the metadata block only requires rewriting the metadata section (and optionally adjusting padding), leaving the audio frame data completely untouched.

---

## Decode Application Path

When decoding PCM samples for a track:
1. Decode the raw audio frame back to PCM.
2. Determine the global sample range of this decoded block.
3. Apply mutes: set samples to 0 in muted ranges.
4. Apply fades: multiply samples by $g(t)$ during fade ranges.
5. Apply gain envelope: multiply samples by the interpolated gain value at each sample index.

```rust
pub fn apply_edits(
    track_id: u32,
    start_sample: u64,
    pcm_buffer: &mut [i32],
    edits: &EditBlock,
) {
    for (i, sample) in pcm_buffer.iter_mut().enumerate() {
        let current_sample = start_sample + i as u64;
        let mut gain = 1.0f32;

        // Apply mute check
        if edits.is_muted(track_id, current_sample) {
            gain = 0.0;
        } else {
            // Apply fade gain
            if let Some(fade_gain) = edits.get_fade_gain(track_id, current_sample) {
                gain *= fade_gain;
            }
            // Apply envelope gain
            gain *= edits.get_envelope_gain(track_id, current_sample);
        }

        // Apply to PCM (clamping to range)
        *sample = (*sample as f32 * gain).round() as i32;
    }
}
```
Since this is applied on-the-fly, no audio re-encoding occurs!
