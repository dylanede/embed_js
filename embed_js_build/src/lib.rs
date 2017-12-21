//! This crate provides functions to call from build and post-build scripts as part of
//! wasm32-unknown-unknown builds that rely on crates using the `embed_js` crate to write inline
//! javascript.
//!
//! See the `embed_js` repository for examples of how to use these crates together.

extern crate embed_js_common;
extern crate cpp_synmap;
extern crate cpp_syn;
extern crate serde_json;
extern crate uuid;
extern crate parity_wasm;

use cpp_synmap::SourceMap;
use cpp_syn::visit::Visitor;
use cpp_syn::{Mac, TokenTree, Delimited};

use parity_wasm::elements::{Module, Section, ExportEntry, Internal};

use std::env;
use std::path::{ PathBuf, Path };
use std::io::{ BufWriter, BufReader, Read };
use std::fs::File;
use std::process::Command;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::collections::HashMap;

use embed_js_common::{ JsMac, JsMacArg };

struct JsVisitor<'a> {
    source_map: &'a mut SourceMap,
    instances: &'a mut Vec<JsMac>,
    included_js: &'a mut String
}
impl<'a> Visitor for JsVisitor<'a> {
    fn visit_mac(&mut self, mac: &Mac) {
        if mac.path.segments.len() != 1 {
            return;
        }
        let tts = match mac.tts[0] {
            TokenTree::Delimited(Delimited { ref tts, .. }, _) => &**tts,
            _ => return,
        };
        match mac.path.segments[0].ident.as_ref() {
            "js" => {
                if let Ok(parsed) = embed_js_common::parse_js_mac_source_map(tts, self.source_map) {
                    self.instances.push(parsed);
                }
            }
            "include_js" => {
                let js_source = if let (Some(first), Some(last)) = (tts.first(), tts.last()) {
                    self.source_map.source_text(first.span().extend(last.span())).unwrap()
                } else {
                    ""
                };
                self.included_js.push_str(&js_source);
                self.included_js.push_str("\n");
            }
            "include" => {
                use cpp_syn::{ Token, Lit, LitKind };
                let mut iter = tts.iter().peekable();
                match iter.next() {
                    Some(&TokenTree::Token(Token::Literal(Lit { node: LitKind::Str(ref path, _), .. }), span)) => {
                        if iter.next().is_some() {
                            return;
                        }
                        let mut path = PathBuf::from(path);
                        if !path.is_absolute() {
                            let root = self.source_map.filename(span).unwrap();
                            path = root.join(path);
                        }
                        println!("cargo:warning=embed_js_build processing source in included file {}", path.display());
                        let krate = self.source_map.add_crate_root(path).unwrap();
                        self.visit_crate(&krate);
                    }
                    Some(&TokenTree::Token(Token::Ident(ref ident), span)) if ident.as_ref() == "concat" => {
                        match iter.next() {
                            Some(&TokenTree::Token(Token::Not, _)) => {}
                            _ => return
                        }
                        let tts = match iter.next() {
                            Some(&TokenTree::Delimited(Delimited { ref tts, .. }, _)) => {
                                tts
                            }
                            _ => return
                        };
                        let mut path = String::new();
                        let mut iter = tts.iter().peekable();
                        while let Some(t) = iter.next() {
                            match *t {
                                TokenTree::Token(Token::Literal(Lit { node: LitKind::Str(ref s, _), .. }), _) => {
                                    path.push_str(s);
                                }
                                TokenTree::Token(Token::Comma, _) => {}
                                TokenTree::Token(Token::Ident(ref ident), _) if ident.as_ref() == "env" => {
                                    match iter.next() {
                                        Some(&TokenTree::Token(Token::Not, _)) => {}
                                        _ => return
                                    }
                                    let tts = match iter.next() {
                                        Some(&TokenTree::Delimited(Delimited { ref tts, .. }, _)) => {
                                            tts
                                        }
                                        _ => return
                                    };
                                    if let Some(&TokenTree::Token(Token::Literal(Lit { node: LitKind::Str(ref s, _), .. }), _)) = tts.first() {
                                        if tts.len() != 1 {
                                            return
                                        }
                                        if let Ok(v) = std::env::var(s) {
                                            path.push_str(&v);
                                        } else {
                                            return
                                        }
                                    } else {
                                        return
                                    }
                                }
                                _ => return
                            }
                        }
                        let mut path = PathBuf::from(path);
                        if !path.is_absolute() {
                            let root = self.source_map.filename(span).unwrap();
                            path = root.join(path);
                        }
                        println!("cargo:warning=embed_js_build processing source in included file {}", path.display());
                        let krate = self.source_map.add_crate_root(path).unwrap();
                        self.visit_crate(&krate);
                    }
                    _ => return
                }
            }
            _ => {}
        }
    }
}

/// Call this once from a build script for a crate that uses `embed_js` directly.
///
/// Parameters:
///
/// * `lib_root` The path to the crate root rust file, e.g. "src/lib.rs"
///
/// Example:
///
/// ```ignore
/// extern crate embed_js_build;
/// fn main() {
///     use std::path::PathBuf;
///     let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("src/lib.rs");
///     embed_js_build::preprocess_crate(&root);
/// }
/// ```
pub fn preprocess_crate(lib_root: &Path) {
    let mut source_map = SourceMap::new();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut instances = Vec::new();
    let mut included_js = String::new();
    let krate = source_map.add_crate_root(lib_root).unwrap();
    JsVisitor {
        source_map: &mut source_map,
        instances: &mut instances,
        included_js: &mut included_js
    }.visit_crate(&krate);

    let js_path = out_dir.join("embed_js_data.json");
    serde_json::to_writer(BufWriter::new(File::create(&js_path).unwrap()), &(instances, included_js)).unwrap();
    let preamble_path = out_dir.join("embed_js_preamble.rs");
    File::create(preamble_path).unwrap();
}

/// Generated from `postprocess_crate`.
pub struct PostProcessData {
    /// The path to the generated wasm binary.
    pub wasm_path: PathBuf,
    /// The contents of the wasm binary, provided for convenience.
    pub wasm: Vec<u8>,
    /// The javascript that should be put as the value of the `env` field in the `importObject`
    /// passed to `WebAssembly.instantiate`.
    pub imports: String,
    /// All javascript specified by the `include_js` macro in linked crates. This should be run
    /// before the WebAssembly module is loaded.
    pub included: String
}
/// Call this once **after** a wasm-unknown-unknown build has completed (i.e. from a post-build
/// script) in order to generate the javascript imports that should accompany the wasm binary.
///
/// See the `embed_js` repository for example projects using this function.
///
/// Parameters:
///
/// * `lib_name` The binary name to process, typically the name of the crate unless set otherwise
///   in `Cargo.toml`.
/// * `debug` Whether to look for the debug or release binary to process. Until wasm32-unkown-unknown
///   supports debug builds, this should always be set to `false`.
///
/// Example post-build script, taken from the "simple" example in the `embed_js` repository:
///
/// ```ignore
/// extern crate base64;
/// extern crate embed_js_build;
///
/// use std::fs::File;
/// use std::io::Write;
///
/// fn main() {
///     let pp_data = embed_js_build::postprocess_crate("simple", false).unwrap();
///     let in_base_64 = base64::encode(&pp_data.wasm);
///     let html_path = pp_data.wasm_path.with_extension("html");
///     let mut html_file = File::create(&html_path).unwrap();
///     write!(html_file, r#"<!DOCTYPE html>
/// <html lang="en">
/// <head>
/// <meta charset="utf-8">
/// <title> wasm test </title>
/// <script>
/// function _base64ToArrayBuffer(base64) {{
///     var binary_string =  window.atob(base64);
///     var len = binary_string.length;
///     var bytes = new Uint8Array( len );
///     for (var i = 0; i < len; ++i) {{
///         bytes[i] = binary_string.charCodeAt(i);
///     }}
///     return bytes.buffer;
/// }}
/// var bytes = _base64ToArrayBuffer(
/// "{}"
/// );
/// WebAssembly.instantiate(bytes, {{ env: {{
/// {}
/// }}}}).then(results => {{
///     window.exports = results.instance.exports;
///     console.log(results.instance.exports.add_two(2));
/// }});
/// </script>
/// </head>
/// </html>
/// "#,
///            in_base_64,
///            pp_data.imports
///     ).unwrap();
/// }
pub fn postprocess_crate(lib_name: &str, debug: bool) -> std::io::Result<PostProcessData> {
    let metadata_json = Command::new("cargo").args(&["metadata", "--format-version", "1"]).output().unwrap().stdout;
    let metadata_json: serde_json::Value = serde_json::from_slice(&metadata_json).unwrap();
    let target_directory = Path::new(metadata_json.as_object().unwrap().get("target_directory").unwrap().as_str().unwrap());
    let bin_prefix = target_directory.join(&format!("wasm32-unknown-unknown/{}/{}", if debug { "debug" } else { "release" }, lib_name));

    // collect json data from all dependency crates
    let d_path = bin_prefix.with_extension("d");
    let mut d_string = String::new();
    File::open(&d_path)?.read_to_string(&mut d_string).unwrap();
    let mut d_pieces: Vec<String> = d_string.split_whitespace().map(String::from).collect::<Vec<_>>();
    { // stick escaped spaces back together
        let mut i = 0;
        while i < d_pieces.len() {
            while d_pieces[i].ends_with("\\") && i != d_pieces.len() - 1 {
                let removed = d_pieces.remove(i+1);
                d_pieces[i].push_str(&removed);
            }
            i += 1;
        }
    }
    d_pieces.remove(0); // remove lib path
    let mut js_macs: HashMap<String, JsMac> = HashMap::new();
    let mut included_js = String::new();
    for path in d_pieces {
        if path.ends_with("out/embed_js_preamble.rs") || path.ends_with("out\\embed_js_preamble.rs") {
            let data_path = PathBuf::from(path).with_file_name("embed_js_data.json");
            let (mut crate_js_macs, crate_included_js): (Vec<JsMac>, String) = serde_json::from_reader(BufReader::new(File::open(data_path)?)).unwrap();
            included_js.push_str(&crate_included_js);
            for js_mac in crate_js_macs.drain(..) {
                let mut hasher = DefaultHasher::new();
                js_mac.hash(&mut hasher);
                let mac_hash = hasher.finish();
                let key = format!("__embed_js__{:x}", mac_hash);
                if let Some(existing) = js_macs.get(&key) {
                    if *existing != js_mac {
                        panic!("A hash collision has occurred in the embed_js build process. Please raise a bug! Meanwhile, try making small changes to your embedded js to remove the collision.")
                    }
                }
                js_macs.insert(key, js_mac);
            }
        }
    }

    let wasm_path = bin_prefix.with_extension("wasm");
    match Command::new("wasm-gc").args(&[&wasm_path, &wasm_path]).output() {
        Ok(output) => {
            if !output.status.success() {
                panic!("wasm-gc encountered an error.\n\nstatus: {}\n\nstdout:\n\n{}\n\nstderr:\n\n{}",
                       output.status,
                       String::from_utf8(output.stdout).unwrap_or_else(|_| String::from("<error decoding stdout>")),
                       String::from_utf8(output.stderr).unwrap_or_else(|_| String::from("<error decoding stderr>")))
            }
        }
        Err(e) => panic!("Error attempting to run wasm-gc. Have you got it installed? Error message: {}", e)
    }
    let mut wasm = Vec::new();
    BufReader::new(File::open(&wasm_path)?).read_to_end(&mut wasm)?;
    let mut module: Module = parity_wasm::deserialize_buffer(wasm.clone()).unwrap();
    // modify the module to export the function table
    let has_table_export = module.export_section()
        .map(|exports| exports.entries()
            .iter()
            .any(|entry| entry.field() == "__table"))
        .unwrap_or(false);
    if !has_table_export && module.table_section().is_some() {
        let sections = module.sections_mut();
        for section in sections {
            match *section {
                Section::Export(ref mut exports) => {
                    exports.entries_mut().push(ExportEntry::new("__table".to_string(), Internal::Table(0)));
                    break;
                }
                _ => {}
            }
        }
    }
    parity_wasm::serialize_to_file(&wasm_path, module.clone()).unwrap();
    wasm.clear();
    BufReader::new(File::open(&wasm_path)?).read_to_end(&mut wasm)?;
    let mut imports = String::new();
    if let Some(import_section) = module.import_section() {
        for entry in import_section.entries() {
            if entry.module() == "env" {
                if let Some(mac) = js_macs.remove(entry.field()) {
                    if !imports.is_empty() {
                        imports.push_str(",\n");
                    }
                    imports.push_str(&format!("{}:function(", entry.field()));
                    let mut start = true;
                    for arg in mac.args {
                        if !start {
                            imports.push_str(", ");
                        } else {
                            start = false;
                        }
                        match arg {
                            JsMacArg::Ref(_, _, name) |
                            JsMacArg::Primitive(_, name, _) => imports.push_str(&name)
                        }
                    }
                    if let Some(body) = mac.body {
                        imports.push_str(&format!("){{{}}}", body));
                    } else {
                        imports.push_str("){}\n");
                    }
                }
            }
        }
    }

    // find
    Ok(PostProcessData {
        wasm_path,
        wasm,
        included: included_js,
        imports
    })
}