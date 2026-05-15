use core::fmt;
use std::fmt::{Debug, Display};
use ulid::{DecodeError, Ulid};


#[derive(Debug, Copy, Clone, Eq, Hash)]
pub struct SUlid(Ulid);

#[derive(Debug)]
pub enum SUlidDecodeError {
    /// The length of the string does not match the expected length
    InvalidLength,
    /// A non-base32 character was found
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


impl SUlid {
    pub fn new() -> Self {
        SUlid(Ulid::new())
    }

    pub fn try_new(s: &str) ->  Result<SUlid, SUlidDecodeError>{
        let ulid = Ulid::from_string(s);
        match ulid {
            Ok(ulid) => Ok(SUlid(ulid)),
            Err(err) => match err {
                DecodeError::InvalidLength => Err(SUlidDecodeError::InvalidLength),
                DecodeError::InvalidChar => Err(SUlidDecodeError::InvalidChar),
            },
        }
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }

}

impl PartialEq for SUlid {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
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
        let s = String::deserialize(deserializer);
        match s {
            Ok(s) => SUlid::try_new(&s).map_err(serde::de::Error::custom),
            Err(err) => Err(err),
        }
    }
}
