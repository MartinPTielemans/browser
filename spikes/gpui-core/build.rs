use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let header = root.join("../../core/public/browser_core.h");
    cc::Build::new()
        .cpp(true)
        .file("host/host.mm")
        .file("host/drive.mm")
        .flag("-std=c++20")
        .flag("-fobjc-arc")
        .define("NDEBUG", None)
        .compile("host");

    // The core's API, read from its header, so the two can't drift.
    bindgen::Builder::default()
        .header(header.to_string_lossy())
        .allowlist_function("bc_.*")
        .allowlist_type("bc_.*")
        .allowlist_var("BC_.*")
        .derive_default(true)
        .rust_edition(bindgen::RustEdition::Edition2024)
        .generate()
        .expect("couldn't read the core's header")
        .write_to_file(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("browser_core.rs"))
        .unwrap();

    let frameworks = root.join(".deps/Chromium.app/Contents/Frameworks");
    println!("cargo:rustc-link-search=framework={}", frameworks.display());
    println!("cargo:rustc-link-lib=framework=Chromium Framework");
    // The dev engine's framework loads from the run path, as Chromium's own executable finds it.
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
    println!("cargo:rustc-link-lib=framework=Cocoa");
    println!("cargo:rerun-if-changed=host");
    println!("cargo:rerun-if-changed={}", header.display());
}
