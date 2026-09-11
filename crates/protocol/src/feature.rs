use core::fmt;

pub const MAX_SUPPORTED_FEATURES: usize = 64;
pub const MAX_REQUIRED_FEATURES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureSet {
    supported: Vec<u16>,
    required: Vec<u16>,
}

impl FeatureSet {
    pub fn new(supported: &[u16], required: &[u16]) -> Result<Self, FeatureNegotiationError> {
        if supported.len() > MAX_SUPPORTED_FEATURES {
            return Err(FeatureNegotiationError::TooManySupportedFeatures);
        }
        if required.len() > MAX_REQUIRED_FEATURES {
            return Err(FeatureNegotiationError::TooManyRequiredFeatures);
        }

        let mut supported = supported.to_vec();
        supported.sort_unstable();
        supported.dedup();

        let mut required = required.to_vec();
        required.sort_unstable();
        required.dedup();

        Ok(Self {
            supported,
            required,
        })
    }

    pub fn supported(&self) -> &[u16] {
        &self.supported
    }

    pub fn required(&self) -> &[u16] {
        &self.required
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureNegotiationError {
    TooManySupportedFeatures,
    TooManyRequiredFeatures,
    UnsupportedRequiredFeature(u16),
}

impl fmt::Display for FeatureNegotiationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManySupportedFeatures => {
                formatter.write_str("too many supported protocol features were advertised")
            }
            Self::TooManyRequiredFeatures => {
                formatter.write_str("too many required protocol features were advertised")
            }
            Self::UnsupportedRequiredFeature(feature) => {
                write!(
                    formatter,
                    "required protocol feature {feature} is unsupported"
                )
            }
        }
    }
}

impl std::error::Error for FeatureNegotiationError {}

pub fn negotiate_features(
    local: &FeatureSet,
    peer: &FeatureSet,
) -> Result<Vec<u16>, FeatureNegotiationError> {
    for &feature in &local.required {
        if peer.supported.binary_search(&feature).is_err() {
            return Err(FeatureNegotiationError::UnsupportedRequiredFeature(feature));
        }
    }
    for &feature in &peer.required {
        if local.supported.binary_search(&feature).is_err() {
            return Err(FeatureNegotiationError::UnsupportedRequiredFeature(feature));
        }
    }

    Ok(local
        .supported
        .iter()
        .copied()
        .filter(|feature| peer.supported.binary_search(feature).is_ok())
        .collect())
}
