// Links chrome.dll, the engine's framework, from the engine setup.sh puts in .deps/engine.
fn main() {
    let engine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.deps/engine");
    println!("cargo:rustc-link-arg={}", engine.join("chrome.dll.lib").display());
    // The name in the window title and the app's menus, as the browser's crate sets it.
    let name = std::fs::read_to_string("../branding/name").expect("no branding/name");
    println!("cargo:rustc-env=APP_NAME={}", name.trim());
    println!("cargo:rerun-if-changed=../branding/name");
    // Its engine is always a prebuilt one, whose header engine/fetch.sh checked (../build.rs).
    println!("cargo::rustc-check-cfg=cfg(engine_header_mismatch)");
}
