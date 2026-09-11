use core::fmt;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RandomError;

impl fmt::Debug for RandomError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RandomError")
    }
}

impl fmt::Display for RandomError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("operating-system secure randomness is unavailable")
    }
}

impl std::error::Error for RandomError {}

pub fn random_bytes<const N: usize>() -> Result<[u8; N], RandomError> {
    let mut bytes = [0_u8; N];
    getrandom::fill(&mut bytes).map_err(|_| RandomError)?;
    Ok(bytes)
}
