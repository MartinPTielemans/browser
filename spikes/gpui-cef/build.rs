use std::path::PathBuf;

fn main() {
    let cef = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".deps/cef");
    cc::Build::new()
        .cpp(true)
        .file("host/host.mm")
        .file("host/drive.mm")
        .include(&cef)
        .flag("-std=c++20")
        .flag("-fobjc-arc")
        // CEF's headers trip -Wextra.
        .extra_warnings(false)
        .define("NDEBUG", None)
        .compile("host");
    println!(
        "cargo:rustc-link-search=native={}",
        cef.join("build/libcef_dll_wrapper").display()
    );
    println!("cargo:rustc-link-lib=static=cef_dll_wrapper");
    println!("cargo:rustc-link-lib=framework=Cocoa");
    println!("cargo:rerun-if-changed=host");
    // setup.sh replaces these when the CEF pin changes.
    println!("cargo:rerun-if-changed=.deps/cef/include/cef_version.h");
    println!(
        "cargo:rerun-if-changed={}",
        cef.join("build/libcef_dll_wrapper/libcef_dll_wrapper.a").display()
    );
}
