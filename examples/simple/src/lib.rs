extern crate subcrate;

pub use subcrate::*;

#[no_mangle]
pub fn add_two(x: i32) -> i32 {
    subcrate::add_one(subcrate::add_one(x))
}
