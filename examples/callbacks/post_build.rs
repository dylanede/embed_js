//! ```cargo
//! [dependencies]
//! base64 = "0.8.0"
//! embed_js_build = "^0.1.3"
//! ```

extern crate base64;
extern crate embed_js_build;

use std::fs::File;
use std::io::Write;

fn main() {
    let pp_data = embed_js_build::postprocess_crate("callbacks", false).unwrap();
    let in_base_64 = base64::encode(&pp_data.wasm);
    let html_path = pp_data.wasm_path.with_extension("html");
    let mut html_file = File::create(&html_path).unwrap();
    write!(html_file, r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title> wasm test </title>
<script>
function _base64ToArrayBuffer(base64) {{
    var binary_string =  window.atob(base64);
    var len = binary_string.length;
    var bytes = new Uint8Array( len );
    for (var i = 0; i < len; ++i) {{
        bytes[i] = binary_string.charCodeAt(i);
    }}
    return bytes.buffer;
}}
var bytes = _base64ToArrayBuffer(
"{}"
);
{}
WebAssembly.instantiate(bytes, {{
env: {{
{}
}}}}).then(results => {{
    window.wasm_instance = results.instance;
    window.wasm_mem = results.instance.exports.memory;
    window.wasm_exports = results.instance.exports;
    window.wasm_table = results.instance.exports.__table;
    document.addEventListener("DOMContentLoaded", function() {{
        results.instance.exports.entry_point();
    }});
}});
</script>
</head>
</html>
"#,
        in_base_64,
        pp_data.included,
        pp_data.imports
    ).unwrap();
}