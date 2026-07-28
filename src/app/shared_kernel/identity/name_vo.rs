use nutype::nutype;

#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 50, regex = r"^[\p{L}0-9 -]+$"),
    derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Display, AsRef)
)]
pub struct NameVo(String);
