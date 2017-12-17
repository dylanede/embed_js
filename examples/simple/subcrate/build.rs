extern crate embed_js_build;

fn main() {
    use std::path::PathBuf;
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("src/lib.rs");
    embed_js_build::preprocess_crate(&root);
}
