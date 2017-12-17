#[macro_use]
extern crate embed_js;

embed_js_preamble!();

pub fn add_one(x: i32) -> i32 {
    js!([x as i32] -> i32 {
        return x + 1;
    })
}