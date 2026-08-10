#![allow(
    non_snake_case,
    non_upper_case_globals,
    non_camel_case_types,
    dead_code,
    clippy::all
)]

mod windows_core {
    pub use ::windows_core::*;

    #[cfg(not(windows))]
    pub trait Free {
        unsafe fn free(&mut self);
    }
}

include!(concat!(env!("OUT_DIR"), "/d3d9_bindings.rs"));

pub use self::windows_core::{BOOL, Error, HRESULT, Interface, Result};
