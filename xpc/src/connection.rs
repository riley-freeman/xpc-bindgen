use std::ffi::os_str;
use std::ffi::CStr;
use std::ffi::OsString;
use std::fmt::Debug;
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;

use ron::Value;
use xpc_bindgen::xpc_connection_send_message;
use xpc_bindgen::xpc_connection_send_message_with_reply_sync;
use xpc_bindgen::xpc_dictionary_create;
use xpc_bindgen::xpc_dictionary_get_string;
use xpc_bindgen::xpc_dictionary_set_string;
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

extern "C" fn main_xpc_event_handler(connection: &XPCConnectionWeak, os_event: xpc_object_t) {
    match connection.upgrade() {
        Some(inner) => {
            let connection: XPCConnection = XPCConnection { inner };
            let event = retrieve_r_data(os_event);
            connection.handle_event(event);
        },
        None => return // TODO: Logging something went wrong
    }
}

fn retrieve_r_data(os_event: xpc_object_t) -> Value {
    let key = OsString::from("R_DATA");
    let value = unsafe {
        let ptr = xpc_dictionary_get_string(
            os_event,
            key.as_bytes().as_ptr() as _
        );
        let value = CStr::from_ptr(ptr);
        String::from_utf8_unchecked(value.to_bytes().to_vec())
    };

    ron::from_str(&value).unwrap()
}

fn build_message_dictionary(message: &Value) -> xpc_object_t {
    let key = OsString::from("R_DATA");

    let data = ron::to_string(message).unwrap();
    let data = OsString::from(data);

    unsafe {
        let dict = xpc_dictionary_create(
            std::ptr::null(), 
            std::ptr::null(),
            0
        );
        xpc_dictionary_set_string (
            dict,
            key.as_bytes().as_ptr() as _,
            data.as_bytes().as_ptr() as _,
        );
        dict
    }
}

type XPCConnectionWeak = Weak<Mutex<XPCConnectionInner>>;

#[derive(Clone)]
pub struct XPCConnection {
    inner: Arc<Mutex<XPCConnectionInner>>,
}

impl XPCConnection {
    pub fn create(n: &str) -> Result<Self, Error> {
        // Make sure the string is zero terminating
        let name = os_str::OsString::from(n);
        let handle = unsafe { xpc_connection_create(
            name.as_bytes().as_ptr() as _, 
            std::ptr::null_mut()
        ) };

        if handle.is_null() {
            return Err(Error::FailedToCreateConnection(String::from(n)))
        }

        Ok(XPCConnection::finish_setup(handle))
    }

    pub fn create_mach_service(n: &str, options: ConnectionOptions) -> Result<Self, Error> {
        // Make sure the string is zero terminating
        let name = os_str::OsString::from(n);
        let handle = unsafe { xpc_connection_create_mach_service(
            name.as_bytes().as_ptr() as _,
            std::ptr::null_mut(),
            options.bits(),
        ) };

        if handle.is_null() {
            return Err(Error::FailedToCreateConnection(String::from(n)))
        }

        Ok(XPCConnection::finish_setup(handle))
    }

    fn finish_setup(handle: xpc_connection_t) -> Self {
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
            main_xpc_event_handler(&connection_weak, os_event);
        });
        unsafe {
            xpc_connection_set_event_handler(
                handle, 
                event_callback.as_ref() as *const _ as *mut _
            );
        }

        connection.inner.lock().unwrap().event_callback = event_callback;
        connection
    }

    pub fn activate(&mut self) {
        let conn = self.inner.lock().unwrap();
        // Apple recommends using active for newer apps
        if crate::utils::VERSION.at_least((10, 12), (10, 0)) {
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

    pub fn send_message(&self, message: &Value) -> Result<(), crate::error::Error> {
        let lock = self.inner.lock().unwrap();

        unsafe {
            let dictionary = build_message_dictionary(message);
            if dictionary.is_null() {
                return Err(Error::DeviceOutOfMemory);
            }

            xpc_connection_send_message(lock.handle, dictionary);
        }

        Ok(())
    }

    // TODO: Something with tokio or smth to make this async maybe
    pub fn send_message_with_reply(&self, message: &Value) -> Result<Value, crate::error::Error> {
        let lock = self.inner.lock().unwrap();

        unsafe {
            let dictionary = build_message_dictionary(message);
            if dictionary.is_null() {
                return Err(Error::DeviceOutOfMemory);
            }
            let xpc_response = xpc_connection_send_message_with_reply_sync(
                lock.handle,
                dictionary
            );

            let response = retrieve_r_data(xpc_response);
            Ok(response)
        }
    }

    fn handle_event(&self, event: Value) {
        let conn = self.inner.lock().unwrap();
        if let Some(delegate) = &conn.delegate {
            delegate.handle_event(event);
        }
    }
}

impl From<xpc_connection_t> for XPCConnection {
    fn from(handle: xpc_connection_t) -> Self {
        Self::finish_setup(handle)   
    }
}

pub trait XPCConnectionDelegate: Debug {
    fn handle_event(&self, event: Value) {
        // Force the event paramater to be used
        let _event = event;
    }
}

