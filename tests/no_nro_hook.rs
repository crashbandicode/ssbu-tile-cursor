//! An rlib alongside the cdylib skips LTO and retains Skyline NRO hook imports.

use std::fs;
use std::path::Path;

fn contains_forbidden(bytes: &[u8], forbidden: &str) -> bool {
    bytes
        .windows(forbidden.len())
        .any(|window| window == forbidden.as_bytes())
}

fn check(path: &Path) {
    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            check(&entry.unwrap().path());
        }
    } else {
        let bytes = fs::read(path).unwrap();
        for forbidden in [
            "add_nro_load_hook",
            "add_nro_unload_hook",
            "skyline::nro",
            "libnro_hook",
        ] {
            assert!(
                !contains_forbidden(&bytes, forbidden),
                "{} contains {forbidden}",
                path.display()
            );
        }
    }
}

#[test]
fn no_nro_hook_provider_in_source_or_manifest() {
    assert!(contains_forbidden(b"\xffadd_nro_load_hook", "add_nro_load_hook"));
    assert!(!contains_forbidden(b"\xffsafe", "add_nro_load_hook"));
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    check(&root.join("src"));
    check(&root.join("Cargo.toml"));
}

#[test]
fn the_plugin_is_built_as_a_cdylib_only() {
    let manifest = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .unwrap();
    let crate_types: Vec<_> = manifest
        .lines()
        .filter(|line| line.trim_start().starts_with("crate-type"))
        .collect();
    assert_eq!(crate_types, ["crate-type = [\"cdylib\"]"]);
}
