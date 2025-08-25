use std::ffi::os_str;
use std::os::unix::ffi::OsStrExt;

use xpc_bindgen::xpc_connection_t;
use xpc_bindgen::xpc_connection_create;
use xpc_bindgen::xpc_connection_create_mach_service;

use xpc_bindgen::xpc_connection_activate;
use xpc_bindgen::xpc_connection_resume;
use xpc_bindgen::xpc_connection_suspend;
use xpc_bindgen::xpc_connection_cancel;

use crate::error::Error;

use bitflags::bitflags;

bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ConnectionOptions: u64 {
        /// A flag that indicates the caller is the listener for the named service.
        const LISTENER = (1 << 0);

        /// A flag that indicates the job advertising the service name belongs to a launch daemon rather than a launch agent.
        /// 
        /// If you use this along side ConnectionOptions::LISTENER, this flag is a no-op.
        const PRIVILEGED = (1 << 1);
    }
}



#[derive(Debug, PartialEq)]
pub struct XPCConnection {
    handle: xpc_connection_t,
}

impl XPCConnection {
    pub fn create(name: &str) -> Result<Self, Error> {
        // Make sure the string is zero terminating
        let name = os_str::OsString::from(name);
        let handle = unsafe { xpc_connection_create(name.as_bytes().as_ptr() as _, std::ptr::null_mut()) };

        Ok( Self {
            handle,
        } )
    }

    pub fn create_mach_service(name: &str, options: ConnectionOptions) -> Result<Self, Error> {
        // Make sure the string is zero terminating
        let name = os_str::OsString::from(name);
        let handle = unsafe { xpc_connection_create_mach_service(
            name.as_bytes().as_ptr() as _,
            std::ptr::null_mut(),
            options.bits(),
        ) };

        Ok( Self {
            handle,
        } )
    }

    pub fn activate(&mut self) {
        // Apple recommends using active for newer apps
        if crate::VERSION.at_least((10, 12), (10, 0)) {
            unsafe { xpc_connection_activate(self.handle); }
        } else {
            unsafe { xpc_connection_resume(self.handle); }
        }
    }

    pub fn resume(&mut self) {
        unsafe { xpc_connection_resume(self.handle); }
    }

    pub fn suspend(&mut self) {
        unsafe { xpc_connection_suspend(self.handle); }
    }

    pub fn cancel(&mut self) {
        unsafe { xpc_connection_cancel(self.handle); }
    }
}

