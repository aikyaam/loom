use crate::edit::schema::TrackEdits;

pub fn apply_edits(pcm: &mut [Vec<i64>], start_sample: u64, edits: &TrackEdits) {
    if pcm.is_empty() {
        return;
    }
    let num_samples = pcm[0].len();
    for i in 0..num_samples {
        let current_sample = start_sample + i as u64;
        let gain = edits.get_gain(current_sample);

        for ch in 0..pcm.len() {
            let sample_val = pcm[ch][i] as f32;
            pcm[ch][i] = (sample_val * gain).round() as i64;
        }
    }
}
