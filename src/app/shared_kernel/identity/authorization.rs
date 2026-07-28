use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum SpaceProfile {
    SpaceAdmin,
    SpaceUser,
}

impl SpaceProfile {
    pub fn as_str(&self) -> &str {
        match self {
            SpaceProfile::SpaceAdmin => "SpaceAdmin",
            SpaceProfile::SpaceUser => "SpaceUser",
        }
    }
}

impl TryFrom<&str> for SpaceProfile {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "SpaceAdmin" => Ok(SpaceProfile::SpaceAdmin),
            "SpaceUser" => Ok(SpaceProfile::SpaceUser),
            other => Err(format!("profil inconnu : {}", other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SpaceProfile;

    #[test]
    fn as_str_round_trips_for_all_variants() {
        for variant in [SpaceProfile::SpaceAdmin, SpaceProfile::SpaceUser] {
            let s = variant.as_str();
            assert_eq!(SpaceProfile::try_from(s).unwrap(), variant);
        }
    }

    #[test]
    fn try_from_valid_strings() {
        assert_eq!(
            SpaceProfile::try_from("SpaceAdmin").unwrap(),
            SpaceProfile::SpaceAdmin
        );
        assert_eq!(
            SpaceProfile::try_from("SpaceUser").unwrap(),
            SpaceProfile::SpaceUser
        );
    }

    #[test]
    fn try_from_unknown_string_returns_err() {
        let result = SpaceProfile::try_from("SuperAdmin");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("profil inconnu"));
    }

    #[test]
    fn try_from_empty_string_returns_err() {
        assert!(SpaceProfile::try_from("").is_err());
    }
}
