// The name in the window title and the app's menus, as the browser's crate sets it. The core's C
// API comes from chrome, which loads this library, so nothing here links the engine.
fn main() {
    let name = std::fs::read_to_string("../branding/name").expect("no branding/name");
    println!("cargo:rustc-env=APP_NAME={}", name.trim());
    println!("cargo:rerun-if-changed=../branding/name");
}
