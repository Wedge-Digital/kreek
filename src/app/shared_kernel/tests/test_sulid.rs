use crate::app::shared_kernel::sulid::SUlid;

#[test]
pub fn assert_sulid_can_be_deserialized_from_valid_str() {
    let valid_string = "01D39ZY06FGSCTVN4T2V9PKHFZ".to_string();
    let sulid = SUlid::try_new(&valid_string);
    assert_eq!(sulid.is_ok(), true);
}

#[test]
pub fn assert_sulid_can_not_be_deserialized_from_string_with_wrong_length() {
    let valid_string = "01D39ZY06FGSCTVN4HFZ".to_string();
    let sulid = SUlid::try_new(&valid_string);
    assert_eq!(sulid.is_err(), true);
    assert_eq!(sulid.err().unwrap().to_string(), "SULID Decode error : invalid length");
}

#[test]
pub fn assert_sulid_can_not_be_deserialized_from_non_string_type() {
    let valid_string = "01D39ZY06FGSCTVN4~~V9PKHFZ".to_string();
    let sulid = SUlid::try_new(&valid_string);
    assert_eq!(sulid.is_err(), true);
    assert_eq!(sulid.err().unwrap().to_string(), "SULID Decode error: invalid character");
}