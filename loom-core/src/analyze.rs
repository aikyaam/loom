pub fn detect_transient(channel_data: &[i64]) -> Option<usize> {
    if channel_data.len() < 512 {
        return None;
    }

    let window_size = 128;
    let mut prev_energy = 0f64;

    for i in 0..window_size {
        let sample = channel_data[i] as f64;
        prev_energy += sample * sample;
    }
    prev_energy /= window_size as f64;
    prev_energy += 1.0;

    for start in (window_size..channel_data.len() - window_size).step_by(window_size / 2) {
        let mut current_energy = 0f64;
        for i in 0..window_size {
            let sample = channel_data[start + i] as f64;
            current_energy += sample * sample;
        }
        current_energy /= window_size as f64;

        if current_energy > prev_energy * 10.0 {
            return Some(start);
        }

        prev_energy = 0.8 * prev_energy + 0.2 * current_energy;
    }

    None
}
