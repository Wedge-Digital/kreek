use nutype::nutype;

#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 200),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct ArticleTitle(String);
