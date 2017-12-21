#[macro_use]
extern crate embed_js;

embed_js_preamble!();

// Some convenience JS for dealing with interop more easily. This sort of stuff should go in a
// higher level crate.
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
    window.get_closure = function(ptr) {
        var array = new Int32Array(wasm_mem.buffer, ptr, 3);
        var f = wasm_table.get(array[0]);
        var data = array[1];
        var drop = array[2];
        var result = function() { f(data); };
        result.drop = function() { drop(data); };
        return result;
    }
}

// MarshalledClosure makes closure interop easier
#[repr(C)]
struct MarshalledClosure {
    call: extern fn(*mut ()),
    data: *mut (),
    drop: extern fn(*mut ())
}

impl MarshalledClosure {
    unsafe fn drop(self) {
        (self.drop)(self.data);
    }
}

unsafe fn marshal_closure<F>(f: F) -> MarshalledClosure
    where F : FnMut()
{
    extern fn call<F>(ptr: *mut ()) where F : FnMut() {
        let f = unsafe { &mut *(ptr as *mut F) };
        f();
    }
    extern fn drop_f<F>(ptr: *mut ()) {
        let f = unsafe { Box::from_raw(ptr as *mut F) };
        drop(f)
    }
    let mut bf = Box::new(f);
    let ptr = &mut *bf as *mut F as *mut ();
    std::mem::forget(bf);
    MarshalledClosure {
        call: call::<F>,
        data: ptr,
        drop: drop_f::<F>
    }
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

    // demonstration of a closure with internal state
    let my_callback = {
        let mut clicked_before = false; // this ends up owned by the closure
        move || {
            if !clicked_before {
                js!({
                    alert("You clicked the button!");
                });
                clicked_before = true;
            } else {
                js!({
                    alert("You clicked the button again!");
                })
            }
        }
    };
    // Closure marshalling:
    // You must ensure that the lifetime bounds of the closure are respected
    // In this case the closure lives for the lifetime of the page (and its bounds allow that),
    // so we don't drop it - if we did we would call c.drop()
    let c = unsafe { marshal_closure(my_callback) };
    // register the event handler
    js!([button_id as i32, &c] {
        var closure = get_closure(c);
        refs[button_id].addEventListener("click", function() {
            closure();
            // if we wanted to drop the closure, we would call closure.drop()
        });
    });

    // drop the reference to the button - not that important in this example but good practice
    js!([button_id as i32] {
        drop_ref(button_id);
    })
}
