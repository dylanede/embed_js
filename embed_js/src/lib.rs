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
/// * `js!([arg1 as type1, arg2 as type2, &arg3, *arg3 as type3, &mut **arg3, ...] -> ret_type { /*javascript*/ })`
///
///   In this form, you specify arguments and a return type. There are two categories of argument.
///
///   Arguments preceded by a `&` are references. These arguments may be any number of reference
///   operations (both immutable and mutable) on an identifier that has been dereferenced via `*`
///   some number of times (including zero). These arguments are passed to the JavaScript as pointers,
///   with type wasm type `i32`. These pointers can then be looked up in the wasm module memory buffer
///   to access the contents of the value referenced.
///
///   Other arguments take the form of a possibly dereferenced identifier followed by `as type`,
///   for some type `type`. Values are cast using `as` to this type before passing to the JavaScript.
///
///   Every type specified, including the return type, must be one of `i32`, `i64`,
///   `f32` or `f64`. These are the only raw types supported by WebAssembly for interop at the
///   moment. More complicated types are best passed by reference.
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
    ([$($args:tt)*] $($tt:tt)*) => {{
        #[derive(EmbedJsDetail)]
        #[allow(dead_code)]
        enum EmbedJsStruct {
            Input = (stringify!([$($args)*] $($tt)*), 0).1
        }
        EmbedJsStruct::call($($args)*)
    }};
    ({$($tt:tt)*}) => {{
        #[derive(EmbedJsDetail)]
        #[allow(dead_code)]
        enum EmbedJsStruct {
            Input = (stringify!({$($tt)*}), 0).1
        }
        EmbedJsStruct::call()
    }};
}

/// Used to specify JavaScript that should be executed before the WebAssembly module is loaded.
/// This is useful for specifying functions that can be shared between instances of inline JS, or
/// set up other global state. This macro is used as a statement or at item level.
///
/// The order of evaluation at runtime is the same as the order of `include_js` calls in the source,
/// with all modules inlined. The order with respect to other crates is not specified.
///
/// Example:
///
/// ```ignore
/// include_js! {
///     window.my_global_function = function() {
///         alert("Hello World!");
///     };
/// }
/// ```
#[macro_export]
macro_rules! include_js {
    ($($tt:tt)*) => {}
}