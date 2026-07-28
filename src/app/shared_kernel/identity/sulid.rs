use core::fmt;
use std::fmt::Display;
use ulid::{DecodeError, Ulid};

#[derive(Debug, Copy, Clone, Eq)]
pub struct SUlid(Ulid);

#[derive(Debug)]
pub enum SUlidDecodeError {
    InvalidLength,
    InvalidChar,
}

impl Display for SUlidDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let text = match *self {
            SUlidDecodeError::InvalidLength => "SULID Decode error : invalid length",
            SUlidDecodeError::InvalidChar => "SULID Decode error: invalid character",
        };
        write!(f, "{}", text)
    }
}

impl Display for SUlid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl SUlid {
    pub fn new() -> Self {
        SUlid(Ulid::new())
    }

    pub fn try_new(s: &str) -> Result<SUlid, SUlidDecodeError> {
        match Ulid::from_string(s) {
            Ok(ulid) => Ok(SUlid(ulid)),
            Err(DecodeError::InvalidLength) => Err(SUlidDecodeError::InvalidLength),
            Err(DecodeError::InvalidChar) => Err(SUlidDecodeError::InvalidChar),
        }
    }
}

impl PartialEq for SUlid {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

// Hash doit être cohérent avec PartialEq — on délègue les deux au Ulid interne.
impl std::hash::Hash for SUlid {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl serde::Serialize for SUlid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.to_string().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for SUlid {
    fn deserialize<D>(deserializer: D) -> Result<SUlid, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        SUlid::try_new(&s).map_err(serde::de::Error::custom)
    }
}
