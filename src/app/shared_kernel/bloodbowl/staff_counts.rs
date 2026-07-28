use serde::{Deserialize, Serialize};

/// Nombre de relances (0–8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RerollCount(pub u8);

impl RerollCount {
    pub const MAX: u8 = 8;
    pub fn new(n: u8) -> Result<Self, &'static str> {
        if n <= Self::MAX {
            Ok(Self(n))
        } else {
            Err("RerollCount max 8")
        }
    }
}

/// Nombre d'assistants d'entraîneur (0–6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AssistantCount(pub u8);

impl AssistantCount {
    pub const MAX: u8 = 6;
    pub fn new(n: u8) -> Result<Self, &'static str> {
        if n <= Self::MAX {
            Ok(Self(n))
        } else {
            Err("AssistantCount max 6")
        }
    }
}

/// Nombre de cheerleaders (0–6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CheerleaderCount(pub u8);

impl CheerleaderCount {
    pub const MAX: u8 = 6;
    pub fn new(n: u8) -> Result<Self, &'static str> {
        if n <= Self::MAX {
            Ok(Self(n))
        } else {
            Err("CheerleaderCount max 6")
        }
    }
}

/// Présence d'un apothicaire (0 ou 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ApothecaryCount(pub u8);

impl ApothecaryCount {
    pub const MAX: u8 = 1;
    pub fn new(n: u8) -> Result<Self, &'static str> {
        if n <= Self::MAX {
            Ok(Self(n))
        } else {
            Err("ApothecaryCount max 1")
        }
    }
    pub fn has(&self) -> bool {
        self.0 == 1
    }
}
