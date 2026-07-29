use nutype::nutype;

#[nutype(
    sanitize(trim),
    validate(not_empty, regex = r"^g[0-9a-z]+$"),
    derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        Hash,
        Serialize,
        Deserialize,
        AsRef,
        Display
    )
)]
pub struct RankingGroupId(String);
