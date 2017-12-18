extern crate proc_macro;

extern crate cpp_syn;
extern crate embed_js_common;
#[macro_use] extern crate quote;
use cpp_syn::{ TokenTree, Ident };

use proc_macro::TokenStream;
use embed_js_common::{ WasmPrimitiveType, JsMacArg };
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
    let mut type_params = Vec::new();
    let mut arg_names = Vec::new();
    let mut arg_types = Vec::new();
    let mut extern_arg_types = Vec::new();
    let mut cast_fragments = Vec::new();
    let mut next_type_param = 0;
    for arg in js_mac.args {
        match arg {
            JsMacArg::Primitive(_, name, ty) => {
                arg_names.push(Ident::from(name));
                let ty = prim_to_ty(ty);
                let ty_ = ty.clone();
                extern_arg_types.push(quote!(#ty_));
                cast_fragments.push(quote!());
                arg_types.push(quote!(#ty));
            }
            JsMacArg::Ref(refs, _, name) => {
                let mutable = refs[0];
                let mutability = if mutable {
                    quote!(mut)
                } else {
                    quote!()
                };
                arg_names.push(Ident::from(name));
                let type_param = Ident::from(format!("T{}", next_type_param));
                {
                    let type_param_ = &type_param;
                    next_type_param += 1;
                    extern_arg_types.push(quote!(*mut u8));
                    arg_types.push(quote!(& #mutability #type_param_));
                    if mutable {
                        cast_fragments.push(quote!(as *mut #type_param_ as *mut u8));
                    } else {
                        cast_fragments.push(quote!(as *const #type_param_ as *const u8 as *mut u8));
                    }
                }
                type_params.push(type_param);
            }
        }
    }
    let type_params = if type_params.len() == 0 {
        quote!()
    } else {
        quote!(<#(#type_params),*>)
    };
    let arg_names = &arg_names;
    let arg_types = &arg_types;
    let extern_name = Ident::from(format!("__embed_js__{:x}", mac_hash));
    let result = match js_mac.ret {
        Some(ty) => {
            let ty = prim_to_ty(ty);
            quote! {
                impl EmbedJsStruct {
                    fn call #type_params(#(#arg_names: #arg_types),*) -> #ty {
                        extern {
                            fn #extern_name(#(#arg_names: #extern_arg_types),*) -> #ty;
                        }
                        unsafe { #extern_name(#(#arg_names #cast_fragments),*) }
                    }
                }
            }
        },
        None => {
            quote! {
                impl EmbedJsStruct {
                    fn call #type_params(#(#arg_names: #arg_types),*) {
                        extern {
                            fn #extern_name(#(#arg_names: #extern_arg_types),*);
                        }
                        unsafe { #extern_name(#(#arg_names #cast_fragments),*) }
                    }
                }
            }
        }
    };
    result.parse().unwrap()
}