//! Extras aimed at storing common items.
use std::collections::HashSet;
use std::{hash::Hash, rc::Rc};

use dasp::frame::Stereo;
use ordered_float::OrderedFloat;
use sealed::sealed;
use slice_dst::SliceWithHeader;

use crate::types::Sound;

/// Trait for sets that contain [`Rc<T>`].
///
/// This is used to cache or deduplicate data, for example resource states.
///
/// # Examples
///
/// ```
/// # use std::collections::HashSet;
/// # use std::rc::Rc;
/// # use mleml::resource::Mod;
/// let mods: HashSet<Rc<[u8]>> = HashSet::new();
/// ```
#[sealed]
pub trait SetRc<T: ?Sized> {
    /// Remove [`Rc`]s that do not exist outside of the set.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::collections::HashSet;
    /// # use std::rc::Rc;
    /// # use serde_json::{json, Value};
    /// # use mleml::resource::JsonArray;
    /// # use mleml::types::Sound;
    /// # use mleml::extra::storage::SetRc;
    /// let mut configs: HashSet<Rc<JsonArray>> = HashSet::new();
    /// let config_1: Rc<JsonArray> = Rc::new(JsonArray::from_value(json!([5, "six"]))
    ///     .expect("failed to create JSON array"));
    /// configs.insert(config_1.clone());
    ///
    /// // A copy of the Rc exists outside of the set
    /// assert_eq!(Rc::strong_count(&config_1), 2);
    ///
    /// drop(config_1);
    /// // Now the Rc is only available in the set, and so will be removed
    /// configs.trim();
    ///
    /// assert!(configs.is_empty());
    /// ```
    fn trim(&mut self);

    /// Store data in the set if it was not already present and return [`Rc<T>`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::collections::HashSet;
    /// # use std::rc::Rc;
    /// # use serde_json::{json, Value};
    /// # use mleml::resource::JsonArray;
    /// # use mleml::resource::ResState;
    /// # use mleml::types::Sound;
    /// # use mleml::extra::storage::SetRc;
    /// let mut states: HashSet<Rc<ResState>> = HashSet::new();
    ///
    /// // These pieces of data are identical
    /// let st1: Box<ResState> = Box::new([0, 0, 15, 3, 6]);
    /// let st2: Box<ResState> = st1.clone();
    ///
    /// let rc1: Rc<ResState> = states.wrap(st1);
    /// // Because data inserted into the set was identical, this Rc is actually a clone of rc1.
    /// let rc2: Rc<ResState> = states.wrap(st2);
    ///
    /// assert_eq!(rc1, rc2);
    /// ```
    fn wrap(&mut self, value: Box<T>) -> Rc<T>;
}

#[sealed]
impl<T: ?Sized + Eq + Hash> SetRc<T> for HashSet<Rc<T>> {
    fn trim(&mut self) {
        self.retain(|r| Rc::strong_count(r) != 1);
    }

    fn wrap(&mut self, value: Box<T>) -> Rc<T> {
        let new = Rc::from(value);
        self.get_or_insert_owned(&new).clone()
    }
}

/// Representation of [`Sound`] that is used to allow storing sound data in `HashSet`.
///
/// This is required because sound data uses floating point numbers which cannot
/// be stored in a set. `OrderedSound` uses `OrderedFloat` instead.
///
/// You won't probably need to use this type directly, see [`wrap_sound()`][SetRcSound::wrap_sound()]
#[derive(Debug, Hash, Eq, PartialEq)]
#[repr(transparent)]
pub struct OrderedSound(SliceWithHeader<usize, Stereo<OrderedFloat<f32>>>);

/// Trait defined for `HashSet<Rc<OrderedSound>>` to allow using it to store [`Sound`] data.
#[sealed]
pub trait SetRcSound {
    /// Store [`Sound`] in the set like [`SetRc::wrap()`].
    ///
    /// Under the hood it stores [`OrderedSound`] and reinterprets it as `Sound`
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::collections::HashSet;
    /// # use std::rc::Rc;
    /// # use serde_json::{json, Value};
    /// # use mleml::resource::JsonArray;
    /// # use mleml::resource::ResState;
    /// # use mleml::types::Sound;
    /// # use mleml::extra::storage::{SetRcSound, OrderedSound};
    /// let mut sounds: HashSet<Rc<OrderedSound>> = HashSet::new();
    ///
    /// // s1 and s2 contain identical data, s3 is unique
    /// let s1: Box<Sound> = Sound::new(Box::new([[0.5, 0.5], [0.6, 0.6]]), 48000);
    /// let s2: Box<Sound> = Sound::new(Box::new([[0.5, 0.5], [0.6, 0.6]]), 48000);
    /// let s3: Box<Sound> = Sound::new(Box::new([[0.1, 0.1], [0.0, 0.0]]), 48000);
    ///
    /// let r1: Rc<Sound> = sounds.wrap_sound(s1);
    /// let r2: Rc<Sound> = sounds.wrap_sound(s2);
    /// let r3: Rc<Sound> = sounds.wrap_sound(s3);
    ///
    /// // r1 and r2 are identical due to s1 and s2 containing identical data
    /// assert_eq!(r1, r2);
    /// // r3 is unique
    /// assert_ne!(r1, r3);
    /// ```
    fn wrap_sound(&mut self, value: Box<Sound>) -> Rc<Sound>;
}

#[sealed]
impl SetRcSound for HashSet<Rc<OrderedSound>> {
    fn wrap_sound(&mut self, value: Box<Sound>) -> Rc<Sound> {
        // SAFETY: OrderedSound and Sound are transparent wrappers around
        // SliceWithHeader<usize, T>, where T is a pair of f32 in one case and a
        // 2x transparent wrapper around f32 in another, meaning that T has identical layout.
        // SliceWithHeader has a defined layout, and thus both types have identical layout.
        unsafe {
            //convert to OrderedSound
            let new = Box::from_raw(Box::into_raw(value) as *mut OrderedSound);
            //store the OrderedSound
            let stored = self.wrap(new);
            //convert back to Sound
            Rc::from_raw(Rc::into_raw(stored) as *const Sound)
        }
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
        let s5: &str = "Five";

        //All Rc's have strong count of 1
        let rc1: Rc<str> = Rc::from(s1);
        let rc2: Rc<str> = Rc::from(s2);
        let rc3: Rc<str> = Rc::from(s3);
        let rc4: Rc<str> = Rc::from(s4);
        let rc5: Rc<str> = Rc::from(s5);

        //Cloned Rc's now have strong count of 2
        let _rc11: Rc<str> = rc1.clone();
        let _rc33: Rc<str> = rc3.clone();
        let _rc55: Rc<str> = rc5.clone();

        let mut set: HashSet<Rc<str>> = HashSet::from([rc1, rc2, rc3, rc4, rc5]);
        assert_eq!(set.len(), 5);
        //Should remove rc2 and rc4
        set.trim();
        //Only rc1, rc3 and rc5 remain as their strong count is 2
        assert_eq!(set.len(), 3);
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
