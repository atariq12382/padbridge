use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceDescriptor {
    pub id: String,
    pub name: String,
    pub vendor_id: u16,
    pub product_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappingConfig {
    pub mappings: Vec<ControlMapping>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlMapping {
    pub source: String,
    pub target: String,
}

impl MappingConfig {
    pub fn from_toml(input: &str) -> Result<Self, MappingError> {
        toml::from_str(input).map_err(MappingError::Parse)
    }
}

#[derive(Debug, Error)]
pub enum MappingError {
    #[error("failed to parse mapping configuration: {0}")]
    Parse(#[from] toml::de::Error),
}