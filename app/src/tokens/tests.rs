//! A colour or a corner radius written into `app/src` instead of `design/tokens.toml` fails the
//! tests, so the design file can't miss a value the UI draws with.

use std::{fs, path::Path};

/// The literals that stay in the code: the file, a piece of the line, and why.
const ALLOWED: &[(&str, &str, &str)] = &[];

#[test]
fn look_values_come_from_the_token_file() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    let mut used = vec![false; ALLOWED.len()];
    for (file, text) in sources(&src) {
        let lines: Vec<&str> = text.lines().collect();
        for (n, line) in lines.iter().enumerate() {
            // A file's tests come last, and their made-up colours aren't the look.
            if line.starts_with("#[cfg(test)]") && lines.get(n + 1).is_some_and(|l| l.starts_with("mod tests {")) {
                break;
            }
            let code = match line.trim_start().starts_with("//") {
                true => "",
                false => line.split(" //").next().unwrap(),
            };
            let Some(literal) = look_literal(code) else { continue };
            match ALLOWED.iter().position(|(f, piece, _)| *f == file && line.contains(piece)) {
                Some(i) => used[i] = true,
                None => found.push(format!("{file}:{}: {literal}", n + 1)),
            }
        }
    }
    assert!(found.is_empty(), "these belong in design/tokens.toml (or in ALLOWED, with why):\n{}", found.join("\n"));
    let stale: Vec<_> = ALLOWED.iter().zip(&used).filter(|(_, used)| !**used).map(|(a, _)| a).collect();
    assert!(stale.is_empty(), "ALLOWED lists literals that are gone: {stale:?}");
}

#[test]
fn finds_colours_and_radii_but_not_tokens() {
    for code in [
        ".bg(rgba(0xffff_ff33))",
        "const FILL: u32 = 0x161b2f;",
        "color: hsla(0., 0., 0., 0.56),",
        ".text_color(rgba(0))",
        ".rounded(px(6.))",
        "corner_radii: Corners::all(px(3.5)),",
    ] {
        assert!(look_literal(code).is_some(), "{code}");
    }
    for code in [
        ".bg(rgba(SELECTED))",
        "hsla(0., 0., 1., TILE_HOVERED)",
        ".rounded(px(PLATE_RADIUS))",
        ".rounded_full()",
        "let flags = 0x0100;",
        "let seed = 0x9e37;",
    ] {
        assert_eq!(look_literal(code), None, "{code}");
    }
}

/// Every Rust file under `dir` but the test files, by its path from there.
fn sources(dir: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut dirs = vec![dir.to_path_buf()];
    while let Some(at) = dirs.pop() {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") && !path.ends_with("tests.rs") {
                let file = path.strip_prefix(dir).unwrap().to_string_lossy().replace('\\', "/");
                out.push((file, fs::read_to_string(&path).unwrap()));
            }
        }
    }
    out
}

/// The first colour or radius written as a number in `code`: a hex colour (six or eight digits),
/// `rgb` or `rgba` of a number, `hsla` of four numbers, or a corner radius of a number.
fn look_literal(code: &str) -> Option<&str> {
    let starts = |at: usize| at == 0 || !code[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_');
    for (at, _) in code.match_indices("0x").filter(|(at, _)| starts(*at)) {
        let digits = code[at + 2..].find(|c: char| !c.is_ascii_hexdigit() && c != '_').map_or(code.len(), |e| at + 2 + e);
        let count = code[at + 2..digits].chars().filter(char::is_ascii_hexdigit).count();
        if count == 6 || count == 8 {
            return Some(&code[at..digits]);
        }
    }
    let number = |arg: &str| {
        let arg = arg.trim().trim_start_matches('-');
        arg.starts_with(|c: char| c.is_ascii_digit()) && arg.chars().all(|c| c.is_ascii_hexdigit() || "._x".contains(c))
    };
    for call in ["rgb", "rgba", "hsla", "rounded", "corner_radii", "Corners::all"] {
        for (at, _) in code.match_indices(call).filter(|(at, _)| starts(*at)) {
            // `rounded` names a family: `rounded_tl` and the rest.
            let name = at + call.len();
            let open = match call {
                "rounded" => name + code[name..].find(|c: char| !c.is_ascii_lowercase() && c != '_').unwrap_or(0),
                _ => name,
            };
            if !code[open..].starts_with('(') {
                continue;
            }
            let close = open + code[open..].find(')').unwrap_or(code.len() - open);
            let args = code[open + 1..close].trim_start_matches("px(");
            let literal = match call {
                "hsla" => args.split(',').count() == 4 && args.split(',').all(number),
                _ => args.split(',').next().is_some_and(number),
            };
            if literal {
                return Some(&code[at..(close + 1).min(code.len())]);
            }
        }
    }
    None
}
