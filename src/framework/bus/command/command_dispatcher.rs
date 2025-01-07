use std::any::{Any, TypeId};
use std::collections::HashMap;
use crate::framework::bus::command::command_type::Command;

pub struct CommandDispatcher {
    executors: HashMap<TypeId, fn(&Command)>
}

impl CommandDispatcher {
    pub fn new() -> Self {
        return CommandDispatcher {
            executors: HashMap::new()
        };
    }

    pub fn register(&mut self, t_id: TypeId, executor: fn(&Command)) {
        self.executors.insert(t_id, executor);
    }

    pub fn dispatch(&self, cmd: &Command) {
        let t_id = cmd.type_id();
        match self.executors.get(&t_id) {
            None => {}
            Some(executor_candidate) => {executor_candidate(cmd)}
        }
    }
}
