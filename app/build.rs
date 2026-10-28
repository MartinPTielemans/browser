use std::path::{Path, PathBuf};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let header = root.join("../core/public/browser_core.h");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();

    let mut host = cc::Build::new();
    host.cpp(true)
        // The driver calls the core for what only agent code will, once there is some.
        .include(root.join("../core/public"))
        .define("NDEBUG", None);
    match os.as_str() {
        "macos" => {
            host.file("platform/mac/host.mm")
                .file("platform/mac/drive.mm")
                .file("platform/mac/menu_window.mm")
                .file("platform/mac/emoji.mm")
                .file("platform/mac/dock.mm")
                .file("platform/mac/updates.mm")
                .flag("-std=c++20")
                .flag("-fobjc-arc")
                // Sparkle's headers alone: updates.mm loads the framework at run time, from a
                // release build that carries it (setup.sh fetches it).
                .flag(format!("-F{}", root.join(".deps/Sparkle").display()));
        }
        "windows" => {
            host.file("platform/windows/host.cc")
                .file("platform/windows/drive.cc")
                .flag("/std:c++20")
                .flag("/utf-8")
                // C++/WinRT (the share sheet) throws, and its projection headers overflow the
                // object file's default section count.
                .flag("/EHsc")
                .flag("/bigobj")
                .define("UNICODE", None)
                .define("_UNICODE", None);
        }
        // Only what the preview asks of the host so far; the browser doesn't build here yet.
        "linux" => {
            host.file("platform/linux/host.cc").flag("-std=c++20");
        }
        other => panic!("no host for {other}"),
    }
    host.compile("host");

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
    if os == "windows" {
        // Chromium's framework on Windows is chrome.dll, which the browser loads from beside it.
        let engine = root.join(".deps/engine");
        println!("cargo:rustc-link-arg-bin=Browser={}", engine.join("chrome.dll.lib").display());
        // windowscodecs is WIC, for host_file_preview's thumbnails; uuid carries the COM GUIDs it
        // and host_drag_file name; windowsapp is WinRT, for the share sheet.
        for lib in ["user32", "gdi32", "dwmapi", "shell32", "ole32", "comctl32", "comdlg32",
                    "windowscodecs", "uuid", "windowsapp"] {
            println!("cargo:rustc-link-lib={lib}");
        }
    } else if os == "macos" {
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
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=CoreText");
        println!("cargo:rustc-link-lib=framework=ImageIO");
        println!("cargo:rustc-link-lib=framework=UniformTypeIdentifiers");
    }

    // A local engine built from another header would run with shifted calls and layouts. Only the
    // browser, which links it, stops on that (main.rs); the tests and the preview don't link it.
    println!("cargo::rustc-check-cfg=cfg(engine_header_mismatch)");
    if let Some(why) = engine_mismatch(&root.join(".deps/Chromium.app"), &header) {
        println!("cargo:rustc-cfg=engine_header_mismatch");
        println!("cargo:rustc-env=ENGINE_HEADER_MISMATCH={why}");
    }

    // The name the user reads, from branding/, which build.sh also reads for the app's bundle.
    let name = root.join("branding/name");
    let name_text = std::fs::read_to_string(&name).expect("no branding/name");
    println!("cargo:rustc-env=APP_NAME={}", name_text.trim());
    println!("cargo:rerun-if-changed={}", name.display());
    println!("cargo:rerun-if-changed=platform");
    println!("cargo:rerun-if-changed={}", header.display());

    // The look's values, from the file the code and the design file share (tools/design-sync.md).
    let tokens = root.join("design/tokens.toml");
    let tokens_text = std::fs::read_to_string(&tokens).expect("no design/tokens.toml");
    let tokens_rs_path = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("tokens.rs");
    std::fs::write(tokens_rs_path, tokens_rs(&tokens_text)).unwrap();
    println!("cargo:rerun-if-changed={}", tokens.display());
}

/// Why the Mac's engine, which setup.sh links at `link`, can't run with bindings from
/// `header`, if it can't: engine/build.sh keeps the header it built from beside it. A prebuilt
/// engine's header was checked by engine/fetch.sh, and Linux and Windows link only those.
fn engine_mismatch(link: &Path, header: &Path) -> Option<String> {
    let engine = std::fs::canonicalize(link).ok()?;
    // A relinked or rebuilt engine checks again, and one without a record every build until it has.
    println!("cargo:rerun-if-changed={}", link.display());
    if engine.components().any(|part| part.as_os_str() == "prebuilt") {
        return None;
    }
    let built = engine.parent()?.join("browser_core.h.built");
    println!("cargo:rerun-if-changed={}", built.display());
    let engine = engine.display();
    match std::fs::read(&built) {
        Ok(built) if built == std::fs::read(header).ok()? => None,
        Ok(_) => Some(format!(
            "the engine at {engine} was built from another core/public/browser_core.h than this \
             checkout's, so the app would call it with shifted functions and layouts. Build it \
             from this checkout with engine/build.sh [dev], or link another with ./setup.sh."
        )),
        Err(_) => Some(format!(
            "the engine at {engine} doesn't say which core/public/browser_core.h it was built \
             from. Run engine/build.sh [dev] to record it."
        )),
    }
}

/// Turns tokens.toml into Rust: a module for each table, a `pub const` for each key, and the
/// comments right above a key as its docs. Integers become `u32` (colours are hex), floats `f32`,
/// and a one-line array of either an array. It reads only that much TOML and stops the build on
/// anything else, so the file can't hold a value the code wouldn't get.
fn tokens_rs(toml: &str) -> String {
    let mut out = String::from("// Generated by build.rs from design/tokens.toml; edit that file.\n");
    let (mut path, mut closed, mut docs) = (Vec::<&str>::new(), Vec::<String>::new(), String::new());
    for (n, line) in toml.lines().enumerate() {
        let line = line.trim();
        let ident = |name: &str| {
            if !name.starts_with(|c: char| c.is_ascii_lowercase())
                || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                fail(n, line, "names are kebab-case");
            }
            name.replace('-', "_")
        };
        let number = |value: &str| match value.strip_prefix("0x") {
            Some(hex) if hex.chars().all(|c| c.is_ascii_hexdigit() || c == '_') => "u32",
            Some(_) => fail(n, line, "not a hex number"),
            None if value.chars().all(|c| c.is_ascii_digit() || c == '_') => "u32",
            None if value.replace('_', "").parse::<f64>().is_ok() => "f32",
            None => fail(n, line, "not a number"),
        };
        if line.is_empty() {
            docs.clear();
        } else if let Some(comment) = line.strip_prefix('#') {
            docs += &format!("///{comment}\n");
        } else if let Some(table) = line.strip_prefix('[').and_then(|line| line.strip_suffix(']')) {
            let table: Vec<&str> = table.split('.').collect();
            let shared = path.iter().zip(&table).take_while(|(a, b)| a == b).count();
            for depth in (shared..path.len()).rev() {
                closed.push(path[..=depth].join("."));
                out += "}\n";
            }
            if shared == table.len() || closed.contains(&table.join(".")) {
                fail(n, line, "a table comes once, with the tables inside it right after it");
            }
            for name in &table[shared..] {
                out += &format!("{docs}pub mod {} {{\n", ident(name));
            }
            (path, docs) = (table, String::new());
        } else if let Some((key, value)) = line.split_once('=') {
            if path.is_empty() {
                fail(n, line, "a key outside a table");
            }
            let value = value.split_once('#').map_or(value, |(value, _)| value).trim();
            let (ty, value) = match value.strip_prefix('[').and_then(|value| value.strip_suffix(']')) {
                Some(items) => {
                    let items: Vec<&str> =
                        items.split(',').map(str::trim).filter(|i| !i.is_empty()).collect();
                    let Some(ty) = items.first().map(|i| number(i)) else { fail(n, line, "an empty array") };
                    if items.iter().any(|i| number(i) != ty) {
                        fail(n, line, "an array of one type");
                    }
                    (format!("[{ty}; {}]", items.len()), format!("[{}]", items.join(", ")))
                }
                None => (number(value).to_string(), value.to_string()),
            };
            let name = ident(key.trim()).to_uppercase();
            out += &format!("{docs}pub const {name}: {ty} = {value};\n");
            docs.clear();
        } else {
            fail(n, line, "not a table, a key or a comment");
        }
    }
    out + &"}\n".repeat(path.len())
}

fn fail(n: usize, line: &str, why: &str) -> ! {
    panic!("design/tokens.toml:{}: {why}: {line}", n + 1)
}
