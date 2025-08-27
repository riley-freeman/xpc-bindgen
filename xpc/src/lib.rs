pub mod connection;
pub mod event;
pub mod error;

#[macro_use]
extern crate objc;

use std::sync::LazyLock;

#[cfg(test)]
mod tests {
    // use super::*;

    // #[test]
    // fn template() {
    // }
}

static VERSION: LazyLock<NSOperatingSystemVersion> = LazyLock::new(|| {
    NSOperatingSystemVersion::default()
});

#[repr(C)]
#[derive(Clone, Copy, Debug)]
#[allow(clippy::upper_case_acronyms)]
struct NSOperatingSystemVersion {
    major: usize,
    minor: usize,
    patch: usize,
}

impl Default for NSOperatingSystemVersion {
    fn default() -> NSOperatingSystemVersion {
        use objc::msg_send;
        use objc::runtime::Object;
        let process_info: *mut Object = unsafe {
            msg_send![class!(NSProcessInfo), processInfo]
        };
        unsafe { msg_send![process_info, operatingSystemVersion] }
    }
}

impl NSOperatingSystemVersion {
    fn at_least(
        &self,
        mac_version: (usize, usize),
        ios_version: (usize, usize),
    ) -> bool {
        #[cfg(target_os = "macos")]
        let is_mac = true;
        #[cfg(not(target_os = "macos"))]
        let is_mac = false;

        if is_mac {
            self.major > mac_version.0
                || (self.major == mac_version.0 && self.minor >= mac_version.1)
        } else {
            self.major > ios_version.0
                || (self.major == ios_version.0 && self.minor >= ios_version.1)
        }
    }
}

