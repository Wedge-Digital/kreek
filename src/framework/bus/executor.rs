pub trait Executor {
    type MessageType;
    type ResponseType;
    type ContextType;
    fn execute(&self, message: &Self::MessageType, ctx: &Self::ContextType) -> Self::ResponseType;
}