extern crate cpp_synmap;
extern crate cpp_syn;
#[macro_use]
extern crate quote;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use std::borrow::Cow;

use cpp_synmap::SourceMap;
use cpp_syn::{TokenTree, Delimited, DelimToken, Token, Span, BinOpToken};

use std::iter::Peekable;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum WasmPrimitiveType {
    I32,
    I64,
    F32,
    F64,
}

fn parse_wasm_primitive_type<'a, I>(iter: &mut Peekable<I>) -> Result<WasmPrimitiveType, ()>
    where
        I: Iterator<Item = &'a TokenTree>,
{
    match iter.next() {
        Some(&TokenTree::Token(Token::Ident(ref ident), _)) => {
            match ident.as_ref() {
                "i32" => Ok(WasmPrimitiveType::I32),
                "i64" => Ok(WasmPrimitiveType::I64),
                "f32" => Ok(WasmPrimitiveType::F32),
                "f64" => Ok(WasmPrimitiveType::F64),
                _ => Err(()),
            }
        }
        _ => Err(()),
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum JsMacArg<'a> {
    Ref(Vec<bool>, usize, Cow<'a, str>),
    Primitive(usize, Cow<'a, str>, WasmPrimitiveType)
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct JsMac<'a> {
    pub args: Vec<JsMacArg<'a>>,
    pub ret: Option<WasmPrimitiveType>,
    pub body: Option<String>,
}

enum SpanJsMacArg {
    Ref(Vec<bool>, usize, Span),
    Primitive(usize, Span, WasmPrimitiveType)
}

struct SpanJsMac {
    args: Vec<SpanJsMacArg>,
    ret: Option<WasmPrimitiveType>,
    body: Option<String>,
}

fn parse_js_mac_span(tts: &[TokenTree]) -> Result<SpanJsMac, ()> {
    let mut iter = tts.iter().peekable();
    let mut args;
    let ret;
    match iter.peek() {
        Some(&&TokenTree::Delimited(Delimited { delim, ref tts }, _)) => {
            match delim {
                DelimToken::Bracket => {
                    iter.next(); // consume
                    args = Vec::new();
                    {
                        let mut iter = tts.iter().peekable();
                        let mut start = true;
                        loop {
                            if start {
                                start = false;
                            } else {
                                match iter.next() {
                                    Some(&TokenTree::Token(Token::Comma, _)) => {},
                                    None => break,
                                    _ => return Err(()),
                                }
                            }
                            if iter.peek().is_none() {
                                break;
                            }
                            let name;
                            let mut derefs = 0;
                            let mut refs = Vec::new();
                            loop {
                                match iter.next() {
                                    Some(&TokenTree::Token(Token::BinOp(BinOpToken::And), _)) => {
                                        if let Some(&&TokenTree::Token(Token::Ident(ref ident), _)) = iter.peek() {
                                            if ident.as_ref() == "mut" {
                                                iter.next();
                                                refs.push(true);
                                            } else {
                                                refs.push(false);
                                            }
                                        } else {
                                            refs.push(false);
                                        }
                                    }
                                    Some(&TokenTree::Token(Token::BinOp(BinOpToken::Star), _)) => {
                                        derefs += 1;
                                    }
                                    Some(&TokenTree::Token(Token::Ident(_), span)) => {
                                        name = span;
                                        break;
                                    },
                                    _ => return Err(())
                                }
                            }
                            println!("Got name");
                            if refs.len() > 0 {
                                args.push(SpanJsMacArg::Ref(refs, derefs, name));
                                println!("Got ref arg");
                            } else {
                                match iter.next() {
                                    Some(&TokenTree::Token(Token::Ident(ref ident), _)) if ident.as_ref() == "as" => {}
                                    _ => return Err(()),
                                }
                                args.push(SpanJsMacArg::Primitive(derefs, name, parse_wasm_primitive_type(&mut iter)?));
                                println!("Got primitive arg");
                            }
                        }
                    }
                    println!("Got args");
                    ret = if let Some(&&TokenTree::Token(Token::RArrow, _)) = iter.peek() {
                        iter.next();
                        Some(parse_wasm_primitive_type(&mut iter)?)
                    } else {
                        None
                    };
                }
                DelimToken::Brace => { // no params or return,
                    args = vec![];
                    ret = None;
                },
                _ => return Err(()),
            }
        }
        _ => return Err(()),
    }

    let result = match iter.next() {
        Some(&TokenTree::Delimited(Delimited {
                                       delim: DelimToken::Brace,
                                       ref tts,
                                   },
                                   _)) => {
            SpanJsMac {
                args,
                ret,
                body: if tts.len() > 0 { Some(quote!(#(#tts)*).to_string()) } else { None },
            }
        }
        _ => return Err(()),
    };
    println!("Done");
    match iter.peek() {
        None => Ok(result),
        Some(_) => Err(())
    }
}

pub fn parse_js_mac_source_map<'a>(tts: &[TokenTree], source_map: &'a SourceMap) -> Result<JsMac<'a>, ()> {
    let spanned = parse_js_mac_span(tts)?;
    Ok(JsMac {
        args: spanned
            .args
            .into_iter()
            .map(|arg| {
                match arg {
                    SpanJsMacArg::Ref(refs, derefs, span) => {
                        JsMacArg::Ref(refs, derefs, Cow::from(source_map.source_text(span).unwrap()))
                    }
                    SpanJsMacArg::Primitive(derefs, span, t) => {
                        JsMacArg::Primitive(derefs, Cow::from(source_map.source_text(span).unwrap()), t)
                    }
                }
            })
            .collect(),
        ret: spanned.ret,
        body: spanned.body,
    })
}

pub fn parse_js_mac_string_source<'a>(tts: &[TokenTree], string_source: &'a str) -> Result<JsMac<'a>, ()> {
    let spanned = parse_js_mac_span(tts)?;
    Ok(JsMac {
        args: spanned
            .args
            .into_iter()
            .map(|arg| {
                match arg {
                    SpanJsMacArg::Ref(refs, derefs, span) => {
                        JsMacArg::Ref(refs, derefs, Cow::from(&string_source[span.lo..span.hi]))
                    }
                    SpanJsMacArg::Primitive(derefs, span, t) => {
                        JsMacArg::Primitive(derefs, Cow::from(&string_source[span.lo..span.hi]), t)
                    }
                }
            })
            .collect(),
        ret: spanned.ret,
        body: spanned.body,
    })
}