// pub struct TestMiddleware {
//     next: Option<Box<TestMiddleware>>,
//     is_handled: bool,
// }
//
// impl TestMiddleware {
//     pub fn new() -> Self {
//         return TestMiddleware { next: None, is_handled: false };
//     }
//
// }
//
// impl CommandMiddleware for TestMiddleware {
//     fn handle<Msg, S: Serialize>(&mut self, message: &Command) -> ProcessingResult<S> {
//         self.is_handled = true;
//         println!("Command message: {:?}", message);
//         let success_body = "Success data".to_string();
//         let success = ProcessingSuccess::new(success_body, "Operation successful".to_string(), 200, "OK".to_string());
//         let processing_result_success: ProcessingResult<String> = Ok(success);
//         return processing_result_success;
//     }
// }
//
// impl Chainable<TestMiddleware> for TestMiddleware {
//     fn set_next(&mut self, next: TestMiddleware) {
//         match self.has_next() {
//             false => { self.next = Some(Box::new(next)) },
//             true => { self.get_next().set_next(next) },
//         }
//     }
//
//     fn get_next(&self) -> &Option<Box<TestMiddleware>> {
//         return &self.next;
//     }
//
//     fn has_next(&self) -> bool {
//         return self.next.is_some();
//     }
// }
//
// impl<Msg:Message> Middleware<Msg> for TestMiddleware {
//     fn handle<M: Message, S: Serialize>(&mut self, message: &M) -> ProcessingResult<S> {
//         self.is_handled = true;
//         return ProcessingResult::Success(S::serialize(message));
//     }
//
// }