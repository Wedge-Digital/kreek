use nutype::nutype;

#[nutype(
    sanitize(trim),
    validate(regex = r"^[\p{L}]{1,3}$"),
    derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Display, AsRef)
)]
pub struct CoachInitials(String);
