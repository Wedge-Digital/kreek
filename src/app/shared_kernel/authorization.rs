
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