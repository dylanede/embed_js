#[macro_use]
extern crate embed_js;

embed_js_preamble!();

// demonstration of using the js macro inside generated source - see the build script
mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated_source.rs"));
}


pub fn add_one(x: i32) -> i32 {
    generated::foo();
    js!([x as i32] -> i32 {
        return x + 1;
    })
}