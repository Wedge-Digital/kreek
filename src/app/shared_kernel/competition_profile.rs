use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum CompetitionProfile {
    CompetitionAdmin,
    CompetitionUser,
}

impl CompetitionProfile {
    pub fn as_str(&self) -> &str {
        match self {
            CompetitionProfile::CompetitionAdmin    => "CompetitionAdmin",
            CompetitionProfile::CompetitionUser => "CompetitionUser",
        }
    }
}

impl TryFrom<&str> for CompetitionProfile {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "CompetitionAdmin"    => Ok(CompetitionProfile::CompetitionAdmin),
            "CompetitionUser"    => Ok(CompetitionProfile::CompetitionUser),
            other           => Err(format!("profil inconnu : {}", other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CompetitionProfile;

    #[test]
    fn as_str_round_trips_for_all_variants() {
        for variant in [CompetitionProfile::CompetitionAdmin,CompetitionProfile::CompetitionUser] {
            let s = variant.as_str();
            assert_eq!(CompetitionProfile::try_from(s).unwrap(), variant);
        }
    }

    #[test]
    fn try_from_valid_strings() {
        assert_eq!(CompetitionProfile::try_from("CompetitionAdmin").unwrap(), CompetitionProfile::CompetitionAdmin);
        assert_eq!(CompetitionProfile::try_from("CompetitionUser").unwrap(), CompetitionProfile::CompetitionUser);
    }

    #[test]
    fn try_from_unknown_string_returns_err() {
        let result = CompetitionProfile::try_from("SuperAdmin");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("profil inconnu"));
    }

    #[test]
    fn try_from_empty_string_returns_err() {
        assert!(CompetitionProfile::try_from("").is_err());
    }
}