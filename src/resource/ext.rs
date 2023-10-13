//TODO: rewrite this doc and others
//!Resources in form of dynamically loaded libraries using C ABI.
//!
//!Important: this assumes that the loaded code is safe. If it segfaults, it will
//!take down the rest of the program.

//TODO: write tests and with that fix many things

use std::{borrow::Cow, ffi::CStr, ptr, rc::Rc, slice};
use dasp::frame::Stereo;
use crate::types::Sound;
use super::{ResConfig, Resource, ResState, Mod, PlatformValues, Platform};

///FFI-friendly immutable slice of PCM data.
#[repr(C)]
pub struct ResSound {
    ///Number of samples per second.
    sampling_rate: u32,
    data_len: usize,
    data: *const Stereo<f32>,
}

impl From<Sound> for ResSound {
    fn from(value: Sound) -> Self {
        ResSound {
            sampling_rate: value.sampling_rate(),
            data_len: value.data().len(),
            data: value.data().as_ptr(),
        }
    }
}

impl Sound {

    ///Create new sound from raw parts.
    pub unsafe fn from_raw_parts(data: *const Stereo<f32>, data_len: usize, sampling_rate: u32) -> Sound {
        Sound::new(
            Box::from(slice::from_raw_parts(data, data_len)),
            sampling_rate
        )
    }

    ///Create new sound from ResSound.
    pub unsafe fn from_res_sound(item: ResSound) -> Self {
        Sound::new (
            Box::from(slice::from_raw_parts(item.data, item.data_len)),
            item.sampling_rate
        )
    }

}

///FFI-friendly return type for all kinds of messages.
///
///Functions like (T, Return<[i8], [i8]>)
#[repr(C)]
struct ResReturn<T: Sized> {
    ///Is the response OK or some kind of an error.
    is_ok: bool,
    ///Returned item.
    item: *const T,
    ///Length of a message.
    msg_len: usize,
    ///Message, interpretation of which depends on `is_ok`.
    msg: *const i8,
}

//I was told this is good
#[repr(C)]
struct NoItem([u8; 0]);

//TODO: wrap dealloc?
///Mod that is loaded at a runtime as a C library.
pub struct ExtMod<I, O> {
    ///Unique ID.
    id: String,

    ///Schema.
    schema: ResConfig,

    ///Pure transformation function.
    apply: extern "C" fn(
        input: *const I,
        conf_size: usize,
        conf: *const u8,
        state_size: usize,
        state: *const u8,
    ) -> ResReturn<O>,

    ///Notify the module that the message can be deallocated safely.
    ///
    ///This is required because the module may have been compiled to use
    ///a different allocator than the library (like jemalloc), which will lead to
    ///issues if Rust side was to deallocate items created by the loaded library.
    dealloc: extern "C" fn(),

    ///Original name of the module.
    orig_name: extern "C" fn() -> *const i8,

    ///Check configuration.
    check_config: extern "C" fn(size: usize, conf: *const u8) -> ResReturn<NoItem>,

    ///Check state.
    check_state: extern "C" fn(size: usize, state: *const u8) -> ResReturn<NoItem>,
    //TODO: this needs to be used during resource creation, it is not necessary
    // to keep around.
    //config_schema: extern "C" fn() -> (u32, *const u8),
}

impl<I, O> Resource for ExtMod<I, O> {
    fn orig_name(&self) -> Option<Cow<'_, str>> {
        unsafe {
            match (self.orig_name)() {
                ptr if ptr.is_null() => None,
                ptr => Some(CStr::from_ptr(ptr).to_string_lossy()),
            }
        }
    }

    fn id(&self) -> &str {
        return self.id.as_str();
    }

    fn check_config(&self, conf: ResConfig) -> Result<(), Cow<'_, str>> {
        let conf = conf.as_byte_vec();
        let ret = (self.check_config)(conf.len(), conf.as_ptr());
        if ret.is_ok {
            return Ok(());
        } else {
            unsafe {
                return Err(CStr::from_ptr(ret.msg).to_string_lossy());
            }
        }
    }

    fn check_state(&self, state: ResState) -> Option<()> {
        (self.check_state)(state.len(), state.as_ptr())
            .is_ok
            .then_some(())
    }
}

//TODO: same as for the prev. block
impl<'msg, I, O> Mod<'msg, I, O> for ExtMod<I, O> {
    fn apply(
        &self,
        input: &I,
        conf: &ResConfig,
        state: ResState,
    ) -> Result<(O, ResState), Cow<'msg, str>> {
        let conf = conf.as_byte_vec();
        unsafe {
            let ret = (self.apply)(
                ptr::from_ref(input),
                conf.len(),
                conf.as_ptr(),
                state.len(),
                state.as_ptr(),
            );
            match ret.is_ok {
                true => Ok((
                    (ret.item as *const O).read(),
                    Rc::from(slice::from_raw_parts(ret.msg as *const u8, ret.msg_len)),
                )),
                false => Err(CStr::from_ptr(ret.msg).to_string_lossy()),
            }
        }
    }
}

///Platform that is loaded at a runtime as a C library.
pub struct ExtPlatform {
    ///Unique ID.
    id: String,

    ///Schema.
    schema: ResConfig,

    ///Get platform values.
    get_vals: extern "C" fn() -> PlatformValues,

    ///Mix provided sound samples.
    mix: extern "C" fn(
        num_channels: usize,
        channels: *const ResSound,
        conf_size: usize,
        conf: *const u8,
        state_size: usize,
        state: *const u8,
    ) -> ResReturn<ResSound>,

    dealloc: extern "C" fn(),

    ///Original name of the module.
    orig_name: extern "C" fn () -> *const i8,

    ///Check configuration.
    check_config: extern "C" fn(size: usize, conf: *const u8) -> ResReturn<NoItem>,

    ///Check state.
    check_state: extern "C" fn(size: usize, state: *const u8) -> ResReturn<NoItem>,
}

impl Resource for ExtPlatform {
    fn orig_name(&self) -> Option<Cow<'_, str>> {
        unsafe {
            match (self.orig_name)() {
                ptr if ptr.is_null() => None,
                ptr => Some(CStr::from_ptr(ptr).to_string_lossy()),
            }
        }
    }

    fn id(&self) -> &str {
        return self.id.as_str();
    }

    fn check_config(&self, conf: ResConfig) -> Result<(), Cow<'_, str>> {
        let conf = conf.as_byte_vec();
        let ret = (self.check_config)(conf.len(), conf.as_ptr());
        if ret.is_ok {
            return Ok(());
        } else {
            unsafe {
                return Err(CStr::from_ptr(ret.msg).to_string_lossy());
            }
        }
    }

    fn check_state(&self, state: ResState) -> Option<()> {
        (self.check_state)(state.len(), state.as_ptr())
            .is_ok
            .then_some(())
    }
}

impl<'msg> Platform<'msg> for ExtPlatform {
    fn get_vals(&self) -> PlatformValues {
        todo!()
    }

    fn mix(&self,
        channels: &[Sound],
        conf: &ResConfig,
        state: ResState,
    ) -> Result<(Sound, ResState), Cow<'msg, str>> {
        let channels: Box<[ResSound]> = channels.into_iter().map(|x| x.to_owned().into()).collect();
        let conf = conf.as_byte_vec();
        unsafe {
            let ret = (self.mix)(
                channels.len(),
                channels.as_ptr(),
                conf.len(),
                conf.as_ptr(),
                state.len(),
                state.as_ptr()
            );
            match ret.is_ok {
                true => Ok((
                Sound::from_res_sound((ret.item as *const ResSound).read()),
                Rc::from(slice::from_raw_parts(ret.msg as *const u8, ret.msg_len)),
                )),
                false => todo!()
            }
        }
    }

    fn description(&self) -> String {
        todo!()
    }
}
