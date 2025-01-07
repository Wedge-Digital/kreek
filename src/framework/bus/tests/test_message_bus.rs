use crate::framework::bus::message::BaseMessage;

#[test]
pub fn assert_assemble_message_bus() {
    let message = BaseMessage::new("Hello".to_string());
    assert_eq!(message.to_string(), "message :: Hello");
}