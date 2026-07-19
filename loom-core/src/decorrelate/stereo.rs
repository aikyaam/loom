#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StereoMode {
    Independent,
    LeftSide,
    RightSide,
    MidSide,
}

impl StereoMode {
    pub fn to_code(&self) -> u8 {
        match self {
            StereoMode::Independent => 0,
            StereoMode::LeftSide => 1,
            StereoMode::RightSide => 2,
            StereoMode::MidSide => 3,
        }
    }

    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(StereoMode::Independent),
            1 => Some(StereoMode::LeftSide),
            2 => Some(StereoMode::RightSide),
            3 => Some(StereoMode::MidSide),
            _ => None,
        }
    }
}

pub fn decorrelate_stereo(left: &[i64], right: &[i64], mode: StereoMode) -> (Vec<i64>, Vec<i64>) {
    let n = left.len();
    let mut ch0 = vec![0i64; n];
    let mut ch1 = vec![0i64; n];

    match mode {
        StereoMode::Independent => {
            ch0.copy_from_slice(left);
            ch1.copy_from_slice(right);
        }
        StereoMode::LeftSide => {
            for i in 0..n {
                ch0[i] = left[i];
                ch1[i] = left[i] - right[i];
            }
        }
        StereoMode::RightSide => {
            for i in 0..n {
                ch0[i] = left[i] - right[i];
                ch1[i] = right[i];
            }
        }
        StereoMode::MidSide => {
            for i in 0..n {
                ch0[i] = (left[i] + right[i]) >> 1;
                ch1[i] = left[i] - right[i];
            }
        }
    }

    (ch0, ch1)
}

pub fn reconstruct_stereo(ch0: &[i64], ch1: &[i64], mode: StereoMode) -> (Vec<i64>, Vec<i64>) {
    let n = ch0.len();
    let mut left = vec![0i64; n];
    let mut right = vec![0i64; n];

    match mode {
        StereoMode::Independent => {
            left.copy_from_slice(ch0);
            right.copy_from_slice(ch1);
        }
        StereoMode::LeftSide => {
            for i in 0..n {
                left[i] = ch0[i];
                right[i] = ch0[i] - ch1[i];
            }
        }
        StereoMode::RightSide => {
            for i in 0..n {
                left[i] = ch0[i] + ch1[i];
                right[i] = ch1[i];
            }
        }
        StereoMode::MidSide => {
            for i in 0..n {
                let mid = ch0[i];
                let side = ch1[i];
                left[i] = mid + ((side + 1) >> 1);
                right[i] = mid - (side >> 1);
            }
        }
    }

    (left, right)
}

pub fn search_stereo_mode(left: &[i64], right: &[i64]) -> (StereoMode, Vec<i64>, Vec<i64>) {
    let modes = [
        StereoMode::Independent,
        StereoMode::LeftSide,
        StereoMode::RightSide,
        StereoMode::MidSide,
    ];

    let mut best_mode = StereoMode::Independent;
    let mut min_sum = u64::MAX;
    let mut best_channels = (Vec::new(), Vec::new());

    for &mode in &modes {
        let (ch0, ch1) = decorrelate_stereo(left, right, mode);

        let mut sum = 0u64;
        for &x in &ch0 {
            sum += x.unsigned_abs();
        }
        for &x in &ch1 {
            sum += x.unsigned_abs();
        }

        if sum < min_sum {
            min_sum = sum;
            best_mode = mode;
            best_channels = (ch0, ch1);
        }
    }

    (best_mode, best_channels.0, best_channels.1)
}
