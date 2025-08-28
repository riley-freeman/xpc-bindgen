use std::sync::LazyLock;

use objc::runtime::Object;

pub static VERSION: LazyLock<NSOperatingSystemVersion> = LazyLock::new(|| {
    NSOperatingSystemVersion::default()
});

#[repr(C)]
#[derive(Clone, Copy, Debug)]
#[allow(clippy::upper_case_acronyms)]
pub struct NSOperatingSystemVersion {
    major: usize,
    minor: usize,
    patch: usize,
}

impl Default for NSOperatingSystemVersion {
    fn default() -> NSOperatingSystemVersion {
        let process_info: *mut Object = unsafe {
            let process_info_class = class!(NSProcessInfo);
            msg_send![process_info_class, processInfo]
        };
        unsafe { msg_send![process_info, operatingSystemVersion] }
    }
}

impl NSOperatingSystemVersion {
    pub fn at_least(
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

