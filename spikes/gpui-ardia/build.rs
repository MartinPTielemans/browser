use std::path::PathBuf;

fn main() {
    let ardia = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".deps/ardia");
    cc::Build::new()
        .cpp(true)
        .file("host/host.mm")
        .file("host/drive.mm")
        .include(ardia.join("include"))
        .flag("-std=c++20")
        .flag("-fobjc-arc")
        .define("NDEBUG", None)
        .compile("host");
    println!("cargo:rustc-link-search=framework={}", ardia.display());
    println!("cargo:rustc-link-lib=framework=Chromium Framework");
    println!("cargo:rustc-link-lib=framework=Cocoa");
    println!("cargo:rerun-if-changed=host");
    // setup.sh replaces the engine when its pin changes.
    println!("cargo:rerun-if-changed={}", ardia.join("tree").display());
}
