//! This crate allows you to embed inline javascript in your Rust code for use with the
//! wasm32-unknown-unknown target. Crates also need to use the `embed_js_build` crate in their
//! build script (see that crate's documentation for more details). Crates that generate binaries
//! must also use `embed_js_build` in a post-build step to collect the generated accompanying
//! javascript from itself and all dependencies.
//!
//! See the documentation pages of the macros in this crate for more details and examples. The
//! embed_js repository also contains example projects.

#[allow(unused_imports)]
#[macro_use]
extern crate embed_js_derive;

#[doc(hidden)]
pub use embed_js_derive::*;

/// For every crate that uses the `js` macro directly, the `embed_js_preamble` macro must be called
/// at least once somewhere in the crate, for example
///
/// ```ignore
/// #[macro_use]
/// extern crate embed_js;
///
/// embed_js_preamble!();
///
/// #[no_mangle]
/// pub fn entry_point() {
///     js!({console.log("Hello world!");});
/// }
/// ```
///
/// You do not need to import `embed_js` at all in crates that only depend on crates that use the
/// `js` macro without directly calling it themselves.
#[macro_export]
macro_rules! embed_js_preamble {
    () => { include!(concat!(env!("OUT_DIR"), "/embed_js_preamble.rs")); }
}

/// Call javascript, inline.
///
/// This macro must not be called from within any other macro, otherwise it will fail to work.
///
/// The javascript written inside calls to this macro must adhere to some additional rules:
///
/// * Every statement must end in a semi-colon.
/// * No single-quote multi-character strings are allowed.
///
/// There are three forms for calling this macro:
///
/// * `js!([arg1 as type1, arg2 as type2, ...] -> ret_type { /*javascript*/ })`
///
///   In this form, you specify arguments and a return type. Every argument must have a type
///   attached to it via `as`. Each argument is cast to this type before being passed to the
///   javascript. Every type specified, including the return type, must be one of `i32`, `i64`,
///   `f32` or `f64`. These are the only raw types supported by WebAssembly for interop at the
///   moment. More complicated types can be passed by using integers to index into memory in the
///   WebAssembly module.
///
///   Examples:
///
///   ```ignore
///   let x = 2;
///   let y = js!([x as i32] -> i32 {
///       return x + 1;
///   });
///   // y is now 3
///   ```
///
///   ```ignore
///   let one_half = js!([] -> f64 {
///       return 1.0 / 2.0;
///   });
///   ```
///
/// * `js!([arg1 as type1, arg2 as type2, ...] { /*javascript*/ })`
///
///   Like the previous form, but without a return type. Any value returned from javascript is
///   discarded.
///
/// * `js!({ /*javascript*/ })`
///
///   No arguments or return type.
#[macro_export]
macro_rules! js {
    ([$($name:ident as $t:ident),*$(,)*] $($tt:tt)*) => {{
        #[derive(EmbedJsDetail)]
        #[allow(dead_code)]
        enum EmbedJsStruct {
            Input = (stringify!([$($name as $t),*] $($tt)*), 0).1
        }
        EmbedJsStruct::call($($name as $t),*)
    }};
    ({$($tt:tt)*}) => {{
        #[derive(EmbedJsDetail)]
        #[allow(dead_code)]
        enum EmbedJsStruct {
            Input = (stringify!($($tt)*), 0).1
        }
        EmbedJsStruct::call()
    }}
}