#[macro_use]
extern crate embed_js;

embed_js_preamble!();

// This macro call sets up some variables before the module is used. They are referred to by
// the inline JS.
include_js! {
    window.refs = new Map();
    window.next_ref_id = 0;
    window.new_ref = function(obj) {
        refs[next_ref_id] = obj;
        var id = next_ref_id;
        next_ref_id += 1;
        return id;
    };
    window.drop_ref = function(id) {
        refs.delete(id);
    };
}

#[no_mangle]
pub fn entry_point() {
    // create a button
    let button_id = js!([] -> i32 {
        var button = document.createElement("button");
        button.appendChild(document.createTextNode("Click me"));
        document.body.appendChild(button);
        return new_ref(button);
    });

    // The rust function to call from the event handler
    extern fn my_callback() {
        js!({
            alert("You clicked the button!");
        });
    }

    // register the event handler
    // all Rust function pointers can be turned into references to wasm functions by looking them up
    // in the __table export of the module, here exposed as wasm_table (see the post build script
    // for how to do this)
    js!([button_id as i32, my_callback as i32] {
        refs[button_id].addEventListener("click", function() {
            wasm_table.get(my_callback)();
        });
    });

    // drop the reference to the button - not that important in this example but good practice
    js!([button_id as i32] {
        drop_ref(button_id);
    })
}