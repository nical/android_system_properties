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
mod android;

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
    properties: android::Properties,
}

impl AndroidSystemProperties {
    /// Create an entry point for accessing Android properties.
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "android")]
            properties: android::Properties::new(),
        }
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
        return self.properties.get(name);
    }
}
