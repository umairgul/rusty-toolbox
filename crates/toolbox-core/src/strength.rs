//! Strength rating for secrets, from their entropy in bits.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Strength {
    Weak,
    Fair,
    Strong,
    VeryStrong,
}

impl Strength {
    pub fn from_bits(bits: f64) -> Self {
        match bits {
            bits if bits < 40.0 => Strength::Weak,
            bits if bits < 60.0 => Strength::Fair,
            bits if bits < 100.0 => Strength::Strong,
            _ => Strength::VeryStrong,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Strength::Weak => "Weak",
            Strength::Fair => "Fair",
            Strength::Strong => "Strong",
            Strength::VeryStrong => "Very strong",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds() {
        assert_eq!(Strength::from_bits(0.0), Strength::Weak);
        assert_eq!(Strength::from_bits(39.9), Strength::Weak);
        assert_eq!(Strength::from_bits(40.0), Strength::Fair);
        assert_eq!(Strength::from_bits(60.0), Strength::Strong);
        assert_eq!(Strength::from_bits(100.0), Strength::VeryStrong);
    }
}
