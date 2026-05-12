
#[derive(Debug, Clone, PartialEq)]
pub enum SpaceAuthorization {
    SpaceAdmin,
    MatchReporter,
    SimpleUser,
}

impl SpaceAuthorization {
    pub fn as_str(&self) -> &str {
        match self {
            SpaceAuthorization::SpaceAdmin    => "SpaceAdmin",
            SpaceAuthorization::MatchReporter => "MatchReporter",
            SpaceAuthorization::SimpleUser    => "SimpleUser",
        }
    }
}

impl TryFrom<&str> for SpaceAuthorization {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "SpaceAdmin"    => Ok(SpaceAuthorization::SpaceAdmin),
            "MatchReporter" => Ok(SpaceAuthorization::MatchReporter),
            "SimpleUser"    => Ok(SpaceAuthorization::SimpleUser),
            other           => Err(format!("profil inconnu : {}", other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SpaceAuthorization;

    #[test]
    fn as_str_round_trips_for_all_variants() {
        for variant in [SpaceAuthorization::SpaceAdmin, SpaceAuthorization::MatchReporter, SpaceAuthorization::SimpleUser] {
            let s = variant.as_str();
            assert_eq!(SpaceAuthorization::try_from(s).unwrap(), variant);
        }
    }

    #[test]
    fn try_from_valid_strings() {
        assert_eq!(SpaceAuthorization::try_from("SpaceAdmin").unwrap(),    SpaceAuthorization::SpaceAdmin);
        assert_eq!(SpaceAuthorization::try_from("MatchReporter").unwrap(), SpaceAuthorization::MatchReporter);
        assert_eq!(SpaceAuthorization::try_from("SimpleUser").unwrap(),    SpaceAuthorization::SimpleUser);
    }

    #[test]
    fn try_from_unknown_string_returns_err() {
        let result = SpaceAuthorization::try_from("SuperAdmin");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("profil inconnu"));
    }

    #[test]
    fn try_from_empty_string_returns_err() {
        assert!(SpaceAuthorization::try_from("").is_err());
    }
}