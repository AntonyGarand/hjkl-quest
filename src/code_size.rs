//! Keeps source files small enough to read in one sitting: a file that
//! grows past the limit is a sign it holds more than one idea, and should
//! be split into a module folder.

use std::fs;
use std::path::Path;

/// The most lines a source file may have (tests included).
const MAX_LINES: usize = 400;

fn long_files(dir: &Path, found: &mut Vec<String>) {
    for entry in fs::read_dir(dir).expect("src is readable") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            long_files(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let lines = fs::read_to_string(&path).expect("a source file").lines().count();
            if lines > MAX_LINES {
                found.push(format!("{} has {lines} lines", path.display()));
            }
        }
    }
}

#[test]
fn source_files_stay_small() {
    let mut found = Vec::new();
    long_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut found);
    assert!(
        found.is_empty(),
        "files over {MAX_LINES} lines:\n{}",
        found.join("\n")
    );
}
