//!Extra things to help with storing data.

use std::collections::HashSet;
use std::{hash::Hash, rc::Rc};

///Trait for sets that contain `Rc<T>`.
///
///This is used to deduplicate cached immutable data, like resource's state,
///or a produced sound.
pub trait SetRc<T> {
    ///Remove unused Rc's.
    fn trim(&mut self);

    ///Return an Rc that already has T or create a new one.
    fn wrap(&mut self, value: T) -> Rc<T>;
}

impl<T: Eq + Hash> SetRc<T> for HashSet<Rc<T>> {
    fn trim(&mut self) {
        self.retain(|r| Rc::strong_count(r) == 1);
    }

    fn wrap(&mut self, value: T) -> Rc<T> {
        let new = Rc::new(value);
        self.get_or_insert_owned(&new).clone()
    }
}
