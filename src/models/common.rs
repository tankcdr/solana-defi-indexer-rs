use serde::{ Deserialize, Serialize };
use std::str::FromStr;

/// Supported DeFi protocols - for extensibility
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Protocol {
    OrcaWhirlpool,
    // Future: RaydiumConcentrated,
    // Future: MercurialStable,
    // etc.
}

impl ToString for Protocol {
    fn to_string(&self) -> String {
        match self {
            Protocol::OrcaWhirlpool => "orca_whirlpool".to_string(),
        }
    }
}

impl FromStr for Protocol {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "orca_whirlpool" => Ok(Protocol::OrcaWhirlpool),
            _ => Err(format!("Unknown protocol: {}", s)),
        }
    }
}

// Note: We don't have a common event struct anymore
// since each protocol will have its own event tables
