//! A thin rust wrapper for Android system properties.
//!
//! This crate is similar to the `android-properties` crate with the exception that
//! the necessary Android libc symbols are loaded dynamically instead of linked
//! statically. In practice this means that the same binary will work with old and
//! new versions of Android, even though the API for reading system properties changed
//! around Android L.
//!
//! ## Example
//!
//! ```rust
//! use android_system_properties::AndroidSystemProperties;
//!
//! let properties = AndroidSystemProperties::new();
//!
//! if let Some(value) = properties.get("persist.sys.timezone") {
//!    println!("{}", value);
//! }
//! ```
//!
//! ## Listing and setting properties
//!
//! For the sake of simplicity this crate currently only contains what's needed by wgpu.
//! The implementations for listing and setting properties can be added back if anyone needs
//! them (let me know by filing an issue).
//!
//! ## License
//!
//! Licensed under either of
//!
//!  * Apache License, Version 2.0 ([LICENSE-APACHE] or <http://www.apache.org/licenses/LICENSE-2.0>)
//!  * MIT license ([LICENSE-MIT] or <http://opensource.org/licenses/MIT>)
//!
//! at your option.
//!
//! [LICENSE-APACHE]: https://github.com/nical/android_system_properties/blob/804681c5c1c93d4fab29c1a2f47b7d808dc70fd3/LICENSE-APACHE
//! [LICENSE-MIT]: https://github.com/nical/android_system_properties/blob/804681c5c1c93d4fab29c1a2f47b7d808dc70fd3/LICENSE-MIT

#[cfg(target_os = "android")]
use std::{
    ffi::{CStr, CString},
    mem,
    os::raw::{c_char, c_int, c_void},
};

#[cfg(target_os = "android")]
unsafe fn property_callback(payload: *mut String, _name: *const c_char, value: *const c_char, _serial: u32) {
    let cvalue = CStr::from_ptr(value);
    (*payload) = cvalue.to_str().unwrap().to_string();
}

#[cfg(target_os = "android")]
type Callback = unsafe fn(*mut String, *const c_char, *const c_char, u32);

#[cfg(target_os = "android")]
type SystemPropertyGetFn = unsafe extern "C" fn(*const c_char, *mut c_char) -> c_int;
#[cfg(target_os = "android")]
type SystemPropertyFindFn = unsafe extern "C" fn(*const c_char) -> *const c_void;
#[cfg(target_os = "android")]
type SystemPropertyReadCallbackFn = unsafe extern "C" fn(*const c_void, Callback, *mut String) -> *const c_void;

#[cfg(target_os = "android")]
#[derive(Debug)]
enum Implementation {
    New {
        find_fn: SystemPropertyFindFn,
        read_callback_fn: SystemPropertyReadCallbackFn,
    },
    Old {
        get_fn: SystemPropertyGetFn,
    }
}

#[cfg(target_os = "android")]
unsafe fn load_fn(libc_so: *mut c_void, cname: &[u8]) -> Option<*const c_void> {
    match libc::dlsym(libc_so, cname.as_ptr().cast()) {
        func if !func.is_null() => Some(func),
        _ => None,
    }
}

#[cfg(target_os = "android")]
impl Implementation {
    unsafe fn load_new(libc_so: *mut c_void) -> Option<Implementation> {
        let read_callback_fn = load_fn(libc_so, b"__system_property_read_callback\0")?;
        let find_fn = load_fn(libc_so, b"__system_property_find\0")?;
        Some(Implementation::New {
            find_fn: mem::transmute(find_fn),
            read_callback_fn: mem::transmute(read_callback_fn),
        })
    }

    unsafe fn load_old(libc_so: *mut c_void) -> Option<Implementation> {
        let get_fn = load_fn(libc_so, b"__system_property_get\0")?;
        Some(Implementation::Old {
            get_fn: mem::transmute(get_fn),
        })
    }

    unsafe fn new(libc_so: *mut c_void) -> Option<Self> {
        Self::load_new(libc_so)
            .or_else(|| Self::load_old(libc_so))
    }

    fn get(&self, cname: *const c_char) -> Option<String> {
        match self {
            Implementation::New { find_fn, read_callback_fn } => {
                let info = unsafe { (find_fn)(cname) };

                if info.is_null() {
                    return None;
                }

                let mut result = String::new();

                unsafe { (read_callback_fn)(info, property_callback, &mut result) };

                Some(result)
            }
            Implementation::Old { get_fn } => {
                // The constant is PROP_VALUE_MAX in Android's libc/include/sys/system_properties.h
                const PROPERTY_VALUE_MAX: usize = 92;
                let mut buffer: Vec<u8> = Vec::with_capacity(PROPERTY_VALUE_MAX);
                let raw = buffer.as_mut_ptr().cast();

                let len = unsafe { (get_fn)(cname, raw) };

                if len > 0 {
                    assert!(len as usize <= buffer.capacity());
                    unsafe { buffer.set_len(len as usize); }
                    String::from_utf8(buffer).ok()
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Debug)]
/// An object that can retrieve android system properties.
///
/// ## Example
///
/// ```
/// use android_system_properties::AndroidSystemProperties;
///
/// let properties = AndroidSystemProperties::new();
///
/// if let Some(value) = properties.get("persist.sys.timezone") {
///    println!("{}", value);
/// }
/// ```
pub struct AndroidSystemProperties {
    #[cfg(target_os = "android")]
    libc_so: *mut c_void,
    #[cfg(target_os = "android")]
    implementation: Option<Implementation>,
}

impl AndroidSystemProperties {
    #[cfg(not(target_os = "android"))]
    /// Create an entry point for accessing Android properties.
    pub fn new() -> Self {
        AndroidSystemProperties {}
    }

    #[cfg(target_os = "android")]
    /// Create an entry point for accessing Android properties.
    pub fn new() -> Self {
        let libc_so = unsafe { libc::dlopen(b"libc.so\0".as_ptr().cast(), libc::RTLD_NOLOAD) };

        let mut properties = AndroidSystemProperties {
            libc_so,
            implementation: None,
        };

        if libc_so.is_null() {
            return properties;
        }

        properties.implementation = unsafe { Implementation::new(libc_so) };

        properties
    }

    /// Retrieve a system property.
    ///
    /// Returns None if the operation fails.
    ///
    /// # Example
    ///
    /// ```
    /// # use android_system_properties::AndroidSystemProperties;
    /// let properties = AndroidSystemProperties::new();
    ///
    /// if let Some(value) = properties.get("persist.sys.timezone") {
    ///     println!("{}", value);
    /// }
    /// ```
    pub fn get(&self, name: &str) -> Option<String> {
        #[cfg(not(target_os = "android"))]
        return (name, None).1;

        #[cfg(target_os = "android")]
        return {
            let implementation = self.implementation.as_ref()?;
            let cname = CString::new(name).ok()?;
            implementation.get(cname.as_ptr().cast())
        };
    }
}

#[cfg(target_os = "android")]
impl Drop for AndroidSystemProperties {
    fn drop(&mut self) {
        if !self.libc_so.is_null() {
            unsafe {
                libc::dlclose(self.libc_so);
            }
        }
    }
}
