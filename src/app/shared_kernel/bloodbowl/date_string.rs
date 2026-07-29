use nutype::nutype;

#[nutype(
    sanitize(trim),
    validate(regex = r"^(?:\d{4}-\d{2}-\d{2})?$"),
    default = "",
    derive(
        Debug,
        Clone,
        Serialize,
        Deserialize,
        PartialEq,
        Eq,
        Display,
        AsRef,
        Default
    )
)]
pub struct DateString(String);
