use std::path::Path;

fn main() {
    let base = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let sources = Path::new(&base).join("site");
    let destination = Path::new(&base).join("target").join("site");
    println!("cargo:rerun-if-changed={}", sources.display());
    hugo_build::init()
        .with_input(sources)
        .with_output(destination)
        .build()
        .unwrap();
}
