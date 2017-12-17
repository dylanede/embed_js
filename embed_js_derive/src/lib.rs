extern crate proc_macro;

extern crate cpp_syn;
extern crate embed_js_common;
#[macro_use] extern crate quote;
use cpp_syn::{ TokenTree, Ident };

use proc_macro::TokenStream;
use embed_js_common::WasmPrimitiveType;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn unwrap_delimited(t: &TokenTree) -> &[TokenTree] {
    match *t {
        TokenTree::Delimited(ref delimited, _) => &delimited.tts,
        _ => panic!("tried to delimit-unwrap a token that isn't a Delimited")
    }
}

fn prim_to_ty(ty: WasmPrimitiveType) -> Ident {
    match ty {
        WasmPrimitiveType::I32 => Ident::from("i32"),
        WasmPrimitiveType::I64 => Ident::from("i64"),
        WasmPrimitiveType::F32 => Ident::from("f32"),
        WasmPrimitiveType::F64 => Ident::from("f64"),
    }
}

#[proc_macro_derive(EmbedJsDetail)]
pub fn embed_js(input: TokenStream) -> TokenStream {
    let s: String = input.to_string();
    let tokens = cpp_syn::parse_token_trees(&s).unwrap();
    let trimmed = unwrap_delimited(&unwrap_delimited(&unwrap_delimited(&tokens[4])[2])[2]);
    let js_mac = embed_js_common::parse_js_mac_string_source(trimmed, &s).expect("syntax error in js macro");
    let mut hasher = DefaultHasher::new();
    js_mac.hash(&mut hasher);
    let mac_hash = hasher.finish();
    let (arg_names, arg_types): (Vec<_>, Vec<_>) = js_mac.args.into_iter()
        .map(|(name, ty)| {
            (Ident::from(name), prim_to_ty(ty))
        })
        .unzip();
    let arg_names = &arg_names;
    let arg_types = &arg_types;
    let extern_name = Ident::from(format!("__embed_js__{:x}", mac_hash));
    let result = match js_mac.ret {
        Some(ty) => {
            let ty = prim_to_ty(ty);
            quote! {
                impl EmbedJsStruct {
                    fn call(#(#arg_names: #arg_types),*) -> #ty {
                        extern {
                            fn #extern_name(#(#arg_names: #arg_types),*) -> #ty;
                        }
                        unsafe { #extern_name(#(#arg_names),*) }
                    }
                }
            }
        },
        None => {
            quote! {
                impl EmbedJsStruct {
                    fn call(#(#arg_names: #arg_types),*) {
                        extern {
                            fn #extern_name(#(#arg_names: #arg_types),*);
                        }
                        unsafe { #extern_name(#(#arg_names),*) }
                    }
                }
            }
        }
    };
    result.parse().unwrap()
}