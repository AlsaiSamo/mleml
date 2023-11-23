//! Extras aimed at storing common items.
use std::collections::HashSet;
use std::{hash::Hash, rc::Rc};

///Trait for sets that contain `Rc<T>`.
///
///This is used to deduplicate cached immutable data, like resource's state,
///or a produced sound.
pub trait SetRc<T: ?Sized> {
    ///Remove unused Rc's.
    fn trim(&mut self);

    ///Return an Rc that already has T or create a new one.
    fn wrap(&mut self, value: Box<T>) -> Rc<T>;
}

impl<T: ?Sized + Eq + Hash> SetRc<T> for HashSet<Rc<T>> {
    fn trim(&mut self) {
        self.retain(|r| Rc::strong_count(r) == 1);
    }

    fn wrap(&mut self, value: Box<T>) -> Rc<T> {
        let new = Rc::from(value);
        self.get_or_insert_owned(&new).clone()
    }
}

#[cfg(test)]
mod tests {
    use std::ptr;

    use super::*;

    #[test]
    fn forgotten_items_are_trimmed() {
        let s1: &str = "One";
        let s2: &str = "Two";
        let s3: &str = "Three";
        let s4: &str = "Four";

        //All Rc's have strong count of 1
        let rc1: Rc<str> = Rc::from(s1);
        let rc2: Rc<str> = Rc::from(s2);
        let rc3: Rc<str> = Rc::from(s3);
        let rc4: Rc<str> = Rc::from(s4);

        //Cloned Rc's now have strong count of 2
        let _rc11: Rc<str> = rc1.clone();
        let _rc33: Rc<str> = rc3.clone();

        let mut set: HashSet<Rc<str>> = HashSet::from([rc1, rc2, rc3, rc4]);
        assert_eq!(set.len(), 4);
        //Should remove rc2 and rc4
        set.trim();
        //Only rc1 and rc3 remain as their strong count is 2
        assert_eq!(set.len(), 2);
    }
    #[test]
    fn wrapping_does_not_insert_duplicate_data() {
        let s1: &str = "One";
        let s2: &str = "One";
        let b1: Box<str> = Box::from(s1);
        let b2: Box<str> = Box::from(s2);
        //We need the pointers to be different
        assert_ne!(b1.as_ptr(), b2.as_ptr());

        let mut set: HashSet<Rc<str>> = HashSet::default();
        let r1 = set.wrap(b1);
        assert_eq!(set.len(), 1);
        //attempt to insert duplicate data
        let r2 = set.wrap(b2);
        //Duplicate was not inserted
        assert_eq!(set.len(), 1);
        //The RC has to be the same one
        assert!(ptr::eq(r1.as_ref(), r2.as_ref()));
        //The RC has to have 3 instances (r1, r2 and in the set)
        assert_eq!(Rc::strong_count(&r2), 3);
    }
}
