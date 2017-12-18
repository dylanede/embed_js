# embed_js

Minimalist lightweight inline JavaScript for Rust applications targeting WebAssembly via the `wasm32-unknown-unknown` target.

This project provides a low level interface designed for other crates to build higher level, 
more ergonomic APIs on top of.

## Getting started

Try out the examples (in the examples subdirectory) to get a feel of how the pieces fit together.

To build wasm applications, make sure you have both a recent Rust nightly and the 
wasm32-unknown-unknown target installed. 

#### Setting up the examples

The examples require [cargo-make](https://github.com/sagiegurari/cargo-make),
[cargo-script](https://github.com/DanielKeep/cargo-script) and [wasm-gc](https://github.com/alexcrichton/wasm-gc) to be installed.

In an example's directory, make sure you have rustup set up to build with nightly Rust.

Build the example using `cargo make`, which ensures that the post-build script is run.
The resulting self-contained HTML file should be in
"target/wasm32-unknown-unknown/release/" and can be ran in a browser (one that supports WebAssembly).

Depending on the example you may need to check the console log in the browser (F12) to see its output.

## General usage

There are two crates to use. `embed_js` is for crates using the `js` macro to embed JavaScript.
`embed_js_build` should be used by those crates as a pre-processing stage in their build scripts. `embed_js_build` should
also be used by application crates that build wasm binaries in their *post*-build scripts in order
to gather the generated accompanying JavaScript to import when loading the wasm module.

See the documentation of both crates for more detailed usage information, or check out the examples in this
repository.

### [embed_js Documentation](https://docs.rs/embed_js)
### [embed_js_build Documentation](https://docs.rs/embed_js)

```toml
[dependencies]
embed_js = "^0.1.3"
```

```toml
[build-dependencies]
embed_js_build = "^0.1.3"
```

## Limitations

Currently the `js` macro cannot be used inside other macros. This is potentially fixable in the future.

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
