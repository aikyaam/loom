#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Apodization {
    Tukey(f64),
    SubdivideTukey(u32),
    PunchoutTukey(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StereoSearch {
    Off,
    Loose,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiceSearch {
    Estimate,
    Limited(u32),
    Exhaustive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionLevel {
    Fast,
    FastPlus,
    Balanced,
    BalancedPlus,
    High,
    HighPlus,
    Insane,
    InsanePlus,
    Maximum,
}

impl CompressionLevel {
    pub fn from_int(level: u8) -> Self {
        match level {
            0 => CompressionLevel::Fast,
            1 => CompressionLevel::FastPlus,
            2 => CompressionLevel::Balanced,
            3 => CompressionLevel::BalancedPlus,
            4 => CompressionLevel::High,
            5 => CompressionLevel::HighPlus,
            6 => CompressionLevel::Insane,
            7 => CompressionLevel::InsanePlus,
            _ => CompressionLevel::Maximum,
        }
    }

    pub fn to_int(&self) -> u8 {
        match self {
            CompressionLevel::Fast => 0,
            CompressionLevel::FastPlus => 1,
            CompressionLevel::Balanced => 2,
            CompressionLevel::BalancedPlus => 3,
            CompressionLevel::High => 4,
            CompressionLevel::HighPlus => 5,
            CompressionLevel::Insane => 6,
            CompressionLevel::InsanePlus => 7,
            CompressionLevel::Maximum => 8,
        }
    }

    pub fn max_lpc_order(&self) -> usize {
        match self {
            CompressionLevel::Fast | CompressionLevel::FastPlus => 0,
            CompressionLevel::Balanced => 6,
            CompressionLevel::BalancedPlus => 8,
            CompressionLevel::High => 12,
            _ => 32,
        }
    }

    pub fn min_partition_order(&self) -> u32 {
        0
    }

    pub fn max_partition_order(&self) -> u32 {
        match self {
            CompressionLevel::Fast => 3,
            CompressionLevel::FastPlus => 3,
            CompressionLevel::Balanced => 4,
            CompressionLevel::BalancedPlus => 5,
            CompressionLevel::High => 6,
            _ => 6,
        }
    }

    pub fn apodizations(&self) -> Vec<Apodization> {
        match self {
            CompressionLevel::Fast | CompressionLevel::FastPlus => {
                vec![Apodization::Tukey(0.5)]
            }
            CompressionLevel::Balanced => {
                vec![Apodization::Tukey(0.5), Apodization::SubdivideTukey(2)]
            }
            CompressionLevel::BalancedPlus | CompressionLevel::High => {
                vec![
                    Apodization::Tukey(0.5),
                    Apodization::SubdivideTukey(2),
                    Apodization::SubdivideTukey(3),
                ]
            }
            _ => {
                vec![
                    Apodization::Tukey(0.5),
                    Apodization::SubdivideTukey(2),
                    Apodization::SubdivideTukey(3),
                    Apodization::PunchoutTukey(3),
                ]
            }
        }
    }

    pub fn stereo_search(&self) -> StereoSearch {
        match self {
            CompressionLevel::Fast => StereoSearch::Off,
            CompressionLevel::FastPlus | CompressionLevel::Balanced => StereoSearch::Loose,
            _ => StereoSearch::Full,
        }
    }

    pub fn rice_search(&self) -> RiceSearch {
        match self {
            CompressionLevel::Fast | CompressionLevel::FastPlus => RiceSearch::Estimate,
            CompressionLevel::Balanced | CompressionLevel::BalancedPlus => RiceSearch::Limited(2),
            _ => RiceSearch::Exhaustive,
        }
    }

    pub fn qlp_precision_search(&self) -> bool {
        match self {
            CompressionLevel::HighPlus
            | CompressionLevel::Insane
            | CompressionLevel::InsanePlus
            | CompressionLevel::Maximum => true,
            _ => false,
        }
    }

    pub fn escape_coding(&self) -> bool {
        match self {
            CompressionLevel::High
            | CompressionLevel::HighPlus
            | CompressionLevel::Insane
            | CompressionLevel::InsanePlus
            | CompressionLevel::Maximum => true,
            _ => false,
        }
    }

    pub fn exhaustive_model_search(&self) -> bool {
        match self {
            CompressionLevel::Maximum => true,
            _ => false,
        }
    }

    pub fn use_double_precision_autocorr(&self) -> bool {
        match self {
            CompressionLevel::Insane | CompressionLevel::InsanePlus | CompressionLevel::Maximum => {
                true
            }
            _ => false,
        }
    }

    pub fn use_irls(&self) -> bool {
        match self {
            CompressionLevel::Insane => true,
            CompressionLevel::InsanePlus | CompressionLevel::Maximum => true,
            _ => false,
        }
    }

    pub fn irls_iterations(&self) -> u32 {
        match self {
            CompressionLevel::Insane => 2,
            CompressionLevel::InsanePlus => 3,
            CompressionLevel::Maximum => 5,
            _ => 0,
        }
    }

    pub fn variable_block_size(&self) -> bool {
        match self {
            CompressionLevel::InsanePlus | CompressionLevel::Maximum => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EncoderConfig {
    pub compression_level: CompressionLevel,
    pub block_size: usize,
    pub sample_rate: u32,
    pub bit_depth: u8,
}

impl EncoderConfig {
    pub fn new(compression_level: u8, block_size: usize, sample_rate: u32, bit_depth: u8) -> Self {
        Self {
            compression_level: CompressionLevel::from_int(compression_level),
            block_size,
            sample_rate,
            bit_depth,
        }
    }

    pub fn default_with_level(level: u8, sample_rate: u32, bit_depth: u8) -> Self {
        let block_size = if level <= 2 { 1152 } else { 4096 };
        Self {
            compression_level: CompressionLevel::from_int(level),
            block_size,
            sample_rate,
            bit_depth,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_level_mapping() {
        for lvl in 0..=8 {
            let cl = CompressionLevel::from_int(lvl);
            assert_eq!(cl.to_int(), lvl);
        }
    }

    #[test]
    fn test_encoder_config_creation() {
        let cfg = EncoderConfig::default_with_level(5, 44100, 16);
        assert_eq!(cfg.block_size, 4096);
        assert_eq!(cfg.sample_rate, 44100);
        assert_eq!(cfg.bit_depth, 16);
    }
}
