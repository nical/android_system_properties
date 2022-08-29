use std::{
    ffi::{CStr, CString},
    mem,
    os::raw::{c_char, c_int, c_void},
};

unsafe fn property_callback(payload: *mut String, _name: *const c_char, value: *const c_char, _serial: u32) {
    let cvalue = CStr::from_ptr(value);
    (*payload) = cvalue.to_str().unwrap().to_string();
}

type Callback = unsafe fn(*mut String, *const c_char, *const c_char, u32);

type SystemPropertyGetFn = unsafe extern "C" fn(*const c_char, *mut c_char) -> c_int;
type SystemPropertyFindFn = unsafe extern "C" fn(*const c_char) -> *const c_void;
type SystemPropertyReadCallbackFn = unsafe extern "C" fn(*const c_void, Callback, *mut String) -> *const c_void;

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

unsafe fn load_fn(libc_so: *mut c_void, cname: &[u8]) -> Option<*const c_void> {
    match libc::dlsym(libc_so, cname.as_ptr().cast()) {
        func if !func.is_null() => Some(func),
        _ => None,
    }
}

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
pub struct Properties {
    libc_so: *mut c_void,
    implementation: Option<Implementation>,
}

impl Properties {
    /// Create an entry point for accessing Android properties.
    pub(crate) fn new() -> Self {
        let libc_so = unsafe { libc::dlopen(b"libc.so\0".as_ptr().cast(), libc::RTLD_NOLOAD) };

        let mut properties = Self {
            libc_so,
            implementation: None,
        };

        if libc_so.is_null() {
            return properties;
        }

        properties.implementation = unsafe { Implementation::new(libc_so) };

        properties
    }

    pub(crate) fn get(&self, name: &str) -> Option<String> {
        let implementation = self.implementation.as_ref()?;
        let cname = CString::new(name).ok()?;
        implementation.get(cname.as_ptr().cast())
    }
}

impl Drop for Properties {
    fn drop(&mut self) {
        if !self.libc_so.is_null() {
            unsafe {
                libc::dlclose(self.libc_so);
            }
        }
    }
}
