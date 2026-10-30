//! Generates the table of embedded web files from the `web/` directory, so
//! adding a file never needs a hand-edited list.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("readable directory entry").path();
        if path.is_dir() {
            collect(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn main() {
    let root = Path::new(&env::var("CARGO_MANIFEST_DIR").expect("set by cargo")).join("web");
    let mut files = Vec::new();
    collect(&root, &mut files);
    files.sort();
    let mut code = String::from("pub const FILES: &[(&str, &[u8])] = &[\n");
    for file in &files {
        let rel = file
            .strip_prefix(&root)
            .expect("under web/")
            .to_string_lossy()
            .replace('\\', "/");
        writeln!(
            code,
            "    ({rel:?}, include_bytes!({:?})),",
            file.display().to_string()
        )
        .unwrap();
    }
    code.push_str("];\n");
    let dest = Path::new(&env::var("OUT_DIR").expect("set by cargo")).join("web_files.rs");
    fs::write(dest, code).expect("write generated table");
    println!("cargo:rerun-if-changed=web");
}
