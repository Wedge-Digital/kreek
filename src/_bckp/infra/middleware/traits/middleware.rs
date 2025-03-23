use crate::app::infra::middleware::traits::app_message::AppMessage;

pub trait Middleware {
        fn execute<T:AppMessage>(&mut self, msg: &mut T) {
            self.handle(msg);

            if let Some(next) = &mut self.next() {
                next.execute(msg);
            }
        }

        fn handle<T:AppMessage>(&mut self, message: &mut T);

        fn next(&mut self) -> &mut Option<Box<dyn Middleware>>;

        fn set_next<T: Middleware>(&mut self, next: T);
}

