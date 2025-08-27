use std::ffi::os_str;
use std::fmt::Debug;
use std::mem;
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;

use xpc_bindgen::xpc_object_t;
use xpc_bindgen::xpc_connection_t;

use xpc_bindgen::xpc_connection_create;
use xpc_bindgen::xpc_connection_create_mach_service;

use xpc_bindgen::xpc_connection_set_event_handler;

use xpc_bindgen::xpc_connection_activate;
use xpc_bindgen::xpc_connection_resume;
use xpc_bindgen::xpc_connection_suspend;
use xpc_bindgen::xpc_connection_cancel;

use crate::error::Error;
use crate::event::XPCEvent;

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


struct XPCConnectionInner {
    handle: xpc_connection_t,
    event_callback: Box<dyn Fn(xpc_object_t) -> ()>,
    delegate: Option<Box<dyn XPCConnectionDelegate>>,
}

extern "C" fn main_xpc_event_handler(connection: XPCConnectionWeak, os_event: xpc_object_t) {
    match connection.upgrade() {
        Some(inner) => {
            let connection: XPCConnection   = XPCConnection { inner };
            let event: XPCEvent             = unsafe { mem::transmute(os_event) };
            connection.handle_event(&event);
        },
        None => return // TODO: Logging something went wrong
    }
}

#[repr(C)]
type XPCConnectionWeak = Weak<Mutex<XPCConnectionInner>>;

#[derive(Clone)]
pub struct XPCConnection {
    inner: Arc<Mutex<XPCConnectionInner>>,
}

impl XPCConnection {
    pub fn create(name: &str) -> Result<Self, Error> {
        // Make sure the string is zero terminating
        let name = os_str::OsString::from(name);
        let handle = unsafe { xpc_connection_create(
            name.as_bytes().as_ptr() as _, 
            std::ptr::null_mut()
        ) };

        XPCConnection::finish_setup(handle)
    }

    pub fn create_mach_service(name: &str, options: ConnectionOptions) -> Result<Self, Error> {
        // Make sure the string is zero terminating
        let name = os_str::OsString::from(name);
        let handle = unsafe { xpc_connection_create_mach_service(
            name.as_bytes().as_ptr() as _,
            std::ptr::null_mut(),
            options.bits(),
        ) };

        XPCConnection::finish_setup(handle)
    }

    fn finish_setup(handle: xpc_connection_t) -> Result<Self, Error> {
        let inner = XPCConnectionInner {
            handle,
            delegate: None,

            #[allow(invalid_value)]
            event_callback: unsafe { MaybeUninit::zeroed().assume_init() },
        };
        let inner = Arc::new(Mutex::new(inner));

        let connection = XPCConnection { inner: Arc::clone(&inner) };
        let connection_weak = Arc::downgrade(&inner);

        let event_callback = Box::new(move |os_event: xpc_object_t| {
            main_xpc_event_handler(connection_weak.clone(), os_event);
        });
        unsafe {
            xpc_connection_set_event_handler(
                handle, 
                event_callback.as_ref() as *const _ as *mut _
            );
        }

        connection.inner.lock().unwrap().event_callback = event_callback;

        Ok(connection)
    }

    pub fn activate(&mut self) {
        let conn = self.inner.lock().unwrap();
        // Apple recommends using active for newer apps
        if crate::VERSION.at_least((10, 12), (10, 0)) {
            unsafe { xpc_connection_activate(conn.handle); }
        } else {
            unsafe { xpc_connection_resume(conn.handle); }
        }
    }

    pub fn resume(&mut self) {
        let conn = self.inner.lock().unwrap();
        unsafe { xpc_connection_resume(conn.handle); }
    }

    pub fn suspend(&mut self) {
        let conn = self.inner.lock().unwrap();
        unsafe { xpc_connection_suspend(conn.handle); }
    }

    pub fn cancel(&mut self) {
        let conn = self.inner.lock().unwrap();
        unsafe { xpc_connection_cancel(conn.handle); }
    }

    fn handle_event(&self, event: &XPCEvent) {
        let conn = self.inner.lock().unwrap();
        if let Some(delegate) = &conn.delegate {
            delegate.handle_event(event);
        }
    }
}

pub trait XPCConnectionDelegate: Debug {
    fn handle_event(&self, event: &XPCEvent) {
        // Force the event paramater to be used
        let _event = event;
    }
}

