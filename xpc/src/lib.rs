pub mod connection;
pub mod event;
pub mod error;

mod utils;

#[macro_use]
pub extern crate objc;


use xpc_bindgen::xpc_connection_t;
use xpc_bindgen::xpc_main;

use crate::connection::XPCConnection;

#[cfg(test)]
mod tests {
    // use super::*;

    // #[test]
    // fn template() {
    // }
}

extern "C" fn main_connection_handler(os_conn: xpc_connection_t) {
    let mut connection = XPCConnection::from(os_conn);
    connection.activate();
}

pub fn main() -> ! {
    unsafe { xpc_main(Some(main_connection_handler)) }
}

