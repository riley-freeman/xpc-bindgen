use std::path::PathBuf;

fn main() {
    #[cfg(target_vendor = "apple")]
    {
        println!("cargo:rerun-if-changed=src/header.h");

        let bindings = bindgen::Builder::default()
            .header("src/header.h")
            .allowlist_var("xpc.*")
            .allowlist_var("_xpc.*")
            .allowlist_var("XPC.*")
            .allowlist_type("uuid_t")
            .allowlist_function("dispatch_queue_create")
            .allowlist_function("xpc.*")
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .generate()
            .expect("Unable to generate bindings");

        bindings
            .write_to_file(PathBuf::from("src/bindings.rs"))
            .expect("Couldn't write bindings!");
    }
}