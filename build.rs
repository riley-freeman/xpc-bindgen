use std::path::PathBuf;

fn main() {
    #[cfg(target_vendor = "apple")]
    {
        println!("cargo:rerun-if-changed=src/header.h");
        println!("cargo:rustc-link-lib=XPC");

        let bindings = bindgen::Builder::default()
            .header("src/header.h")
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .generate()
            .expect("Unable to generate bindings");

        bindings
            .write_to_file(PathBuf::from("src/bindings.rs"))
            .expect("Couldn't write bindings!");
    }
}