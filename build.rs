use std::{env, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=data/io.github.krteke.Waddle.gschema.xml");

    let output = env::var("OUT_DIR").unwrap();

    let status = Command::new("glib-compile-schemas")
        .args(["--strict", "--targetdir", &output, "data"])
        .status()
        .expect("glib-compile-schemas must be installed");

    assert!(status.success(), "GSettings schema compilation failed");
}
