fn main() {
    // This is for macOS. It tells the linker to allow undefined symbols,
    // which will be resolved by the Python interpreter at runtime.
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-undefined");
        println!("cargo:rustc-link-arg=dynamic_lookup");
    }
}