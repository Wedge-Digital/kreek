
pub trait Chainable {
    fn set_next<C: Chainable>(&mut self, next: &C);

    fn get_next<C: Chainable>(&self) -> Option<C>;
}