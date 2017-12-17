#[macro_use]
extern crate embed_js;

embed_js_preamble!();

use std::mem;

#[no_mangle]
pub fn alloc(n: i32) -> *mut u8 {
    let mut v = Vec::with_capacity(n as usize);
    let ptr = v.as_mut_ptr();
    mem::forget(v);
    ptr
}

#[no_mangle]
pub fn free(ptr: *mut u8, len: i32) {
    drop(unsafe { Vec::from_raw_parts(ptr, 0, len as usize) })
}

// not actually used in this example, but included for completeness
#[no_mangle]
pub fn realloc(ptr: *mut u8, old_len: i32, new_len: i32) -> *mut u8 {
    let mut v = unsafe{Vec::from_raw_parts(ptr, 0, old_len as usize)};
    v.reserve_exact(new_len as usize);
    let ptr = v.as_mut_ptr();
    mem::forget(v);
    ptr
}

#[no_mangle]
pub fn entry_point() {
    // Rust str to JS
    {
        let s = "Hello JS!";
        let s = s.as_bytes();
        let p = s.as_ptr();
        let l = s.len();
        js!([p as i32, l as i32] {
            var array = new Uint8Array(wasm_mem.buffer, p, l);
            var string = (new TextDecoder("utf-8")).decode(array);
            alert("str from Rust: " + string);
        });
    }

    // JS String to Rust String
    let rust_string;
    {
        let mut l: u32  = 0;
        let l_ptr = &mut l as *mut _;
        let p = js!([l_ptr as i32] -> i32 {
            var js_string = "Hello Rust!";
            // TextDecoder doesn't support decoding into an existing buffer yet :(
            var array = (new TextEncoder("utf-8")).encode(js_string);
            // allocate a Vec<u8>
            var ptr = wasm_exports.alloc(array.length);
            var rust_array = new Uint8Array(wasm_mem.buffer, ptr, array.length);
            rust_array.set(array);
            // writing into a *mut u32
            var rust_len = new Uint32Array(wasm_mem.buffer, l_ptr, 1);
            rust_len[0] = array.length;
            return ptr;
        }) as *mut u8;
        rust_string = unsafe { String::from_utf8_unchecked(Vec::from_raw_parts(p, l as usize, l as usize)) };
    }

    // Rust String to JS
    {
        let s = rust_string.replace("Rust", "again JS");
        let s = s.into_bytes();
        let p = s.as_ptr();
        let l = s.len();
        let c = s.capacity();
        mem::forget(s);
        js!([p as i32, l as i32, c as i32] {
            var array = new Uint8Array(wasm_mem.buffer, p, l);
            var string = (new TextDecoder("utf-8")).decode(array);
            alert("String from Rust: " + string);
            wasm_exports.free(p, c);
        });
    }
}