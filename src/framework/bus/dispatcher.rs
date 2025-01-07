pub trait Dispatcher {
    type MessageType;
    type ResponseType;
    fn dispatch(&self, message: &Self::MessageType) -> Self::ResponseType;

    // fn register(typ: TypeId, executor: , executor_map: HashMap<TypeId, Box<dyn Command>>);
}