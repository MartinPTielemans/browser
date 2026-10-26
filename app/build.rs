use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let header = root.join("../core/public/browser_core.h");
    cc::Build::new()
        .cpp(true)
        .file("platform/mac/host.mm")
        .file("platform/mac/drive.mm")
        // The driver calls the core for what only agent code will, once there is some.
        .include(root.join("../core/public"))
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

    // Only the browser links the engine; the preview runs without it.
    let frameworks = root.join(".deps/Chromium.app/Contents/Frameworks");
    for arg in [
        format!("-F{}", frameworks.display()),
        "-framework".into(),
        "Chromium Framework".into(),
        // The dev engine's framework loads from the run path, as Chromium's own executable finds it.
        "-Wl,-rpath,@executable_path/../Frameworks".into(),
    ] {
        println!("cargo:rustc-link-arg-bin=Browser={arg}");
    }
    println!("cargo:rustc-link-lib=framework=Cocoa");
    println!("cargo:rustc-link-lib=framework=ImageIO");
    println!("cargo:rustc-link-lib=framework=UniformTypeIdentifiers");

    // The name the user reads, from branding/, which build.sh also reads for the app's bundle.
    let name = root.join("branding/name");
    let name_text = std::fs::read_to_string(&name).expect("no branding/name");
    println!("cargo:rustc-env=APP_NAME={}", name_text.trim());
    println!("cargo:rerun-if-changed={}", name.display());
    println!("cargo:rerun-if-changed=platform");
    println!("cargo:rerun-if-changed={}", header.display());
}
