#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FadeShape {
    Linear = 0,
    SCurve = 1,
    Exponential = 2,
}

impl FadeShape {
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(FadeShape::Linear),
            1 => Some(FadeShape::SCurve),
            2 => Some(FadeShape::Exponential),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MuteRegion {
    pub start_sample: u64,
    pub end_sample: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fade {
    pub start_sample: u64,
    pub end_sample: u64,
    pub shape: FadeShape,
    pub is_fade_in: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GainPoint {
    pub sample_offset: u64,
    pub gain: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackEdits {
    pub mutes: Vec<MuteRegion>,
    pub fades: Vec<Fade>,
    pub gain_points: Vec<GainPoint>,
}

impl TrackEdits {
    pub fn new() -> Self {
        Self {
            mutes: Vec::new(),
            fades: Vec::new(),
            gain_points: Vec::new(),
        }
    }

    pub fn get_gain(&self, sample: u64) -> f32 {
        for mute in &self.mutes {
            if sample >= mute.start_sample && sample < mute.end_sample {
                return 0.0;
            }
        }

        let mut gain = 1.0f32;

        for fade in &self.fades {
            if sample >= fade.start_sample && sample < fade.end_sample {
                let range = (fade.end_sample - fade.start_sample) as f64;
                if range > 0.0 {
                    let progress = (sample - fade.start_sample) as f64 / range;
                    let factor = match fade.shape {
                        FadeShape::Linear => {
                            if fade.is_fade_in {
                                progress
                            } else {
                                1.0 - progress
                            }
                        }
                        FadeShape::SCurve => {
                            let x = progress * std::f64::consts::PI;
                            if fade.is_fade_in {
                                0.5 * (1.0 - x.cos())
                            } else {
                                0.5 * (1.0 + x.cos())
                            }
                        }
                        FadeShape::Exponential => {
                            if fade.is_fade_in {
                                10.0f64.powf(2.0 * (progress - 1.0))
                            } else {
                                10.0f64.powf(-2.0 * progress)
                            }
                        }
                    };
                    gain *= factor as f32;
                }
            }
        }

        if !self.gain_points.is_empty() {
            let pt = &self.gain_points[0];
            if sample <= pt.sample_offset {
                gain *= pt.gain;
            } else if sample >= self.gain_points.last().unwrap().sample_offset {
                gain *= self.gain_points.last().unwrap().gain;
            } else {
                for i in 0..self.gain_points.len() - 1 {
                    let p0 = &self.gain_points[i];
                    let p1 = &self.gain_points[i + 1];
                    if sample >= p0.sample_offset && sample < p1.sample_offset {
                        let t = (sample - p0.sample_offset) as f32
                            / (p1.sample_offset - p0.sample_offset) as f32;
                        let val = p0.gain + (p1.gain - p0.gain) * t;
                        gain *= val;
                        break;
                    }
                }
            }
        }

        gain.clamp(0.0, 2.0)
    }
}
