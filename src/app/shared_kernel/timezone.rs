use nutype::nutype;

#[nutype(
    sanitize(trim),
    validate(regex = r"^(?:[A-Za-z][A-Za-z0-9_/+\-:]*)?$"),
    default = "",
    derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Display, AsRef, Default)
)]
pub struct Timezone(String);
