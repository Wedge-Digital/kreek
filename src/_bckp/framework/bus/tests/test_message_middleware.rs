
#[test]
pub fn assert_instanciate_message_middleware() {
    // let mut middleware_1 = TestMiddleware::new();
    // let middleware_2 = TestMiddleware::new();
    // let middleware_2_id = middleware_2.type_id();
    // assert_eq!(middleware_2.get_next().is_none(), true);
    // middleware_1.set_next(middleware_2);
    // assert_eq!(middleware_1.get_next().is_some(), true);
    // assert_eq!(middleware_1.get_next().unwrap().get_next().is_none(), true);
    // assert_eq!(middleware_1.get_next().unwrap().type_id(), middleware_2_id);
}

#[test]
pub fn assert_handle_message_middleware() {
    // let mut middleware_1 = TestMiddleware::new();
    // let message = "Hello".to_string();
    // let handled_message = middleware_1.handle(BaseMessage::new(message));
    // assert_eq!(middleware_1.is_handled(), true);
    // assert_eq!(handled_message.to_string(), "message :: Hello");
}

#[test]
pub fn assert_chain_handle_message_middleware() {
    // let mut middleware_1 = TestMiddleware::new();
    // let mut middleware_2 = TestMiddleware::new();
    // let message_1= "Hello".to_string();
    // let message_2= "Hello 2".to_string();
    // let handled_message = middleware_1.handle(BaseMessage::new(message_1));
    // assert_eq!(middleware_1.is_handled(), true);
    // assert_eq!(middleware_2.is_handled(), false);
    // assert_eq!(handled_message.to_string(), "message :: Hello");
    // middleware_1.set_next(middleware_2);
    // let handled_message = middleware_1.handle(BaseMessage::new(message_2));
    // assert_eq!(middleware_1.is_handled(), true);
    // assert_eq!(middleware_1.get_next().unwrap().is_handled(), true);
    // assert_eq!(handled_message.to_string(), "message :: Hello 2");
}

#[test]
pub fn assert_last_action_after_chain_handle_is_dispatch() {
}