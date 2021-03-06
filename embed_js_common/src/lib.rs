extern crate cpp_synmap;
extern crate cpp_syn;
#[macro_use]
extern crate quote;
extern crate serde;
#[macro_use]
extern crate serde_derive;

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
pub enum JsMacArg {
    Ref(Vec<bool>, usize, String),
    Primitive(usize, String, WasmPrimitiveType)
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct JsMac {
    pub args: Vec<JsMacArg>,
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
                            if refs.len() > 0 {
                                args.push(SpanJsMacArg::Ref(refs, derefs, name));
                            } else {
                                match iter.next() {
                                    Some(&TokenTree::Token(Token::Ident(ref ident), _)) if ident.as_ref() == "as" => {}
                                    _ => return Err(()),
                                }
                                args.push(SpanJsMacArg::Primitive(derefs, name, parse_wasm_primitive_type(&mut iter)?));
                            }
                        }
                    }
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
    match iter.peek() {
        None => Ok(result),
        Some(_) => Err(())
    }
}

pub fn parse_js_mac_source_map(tts: &[TokenTree], source_map: &SourceMap) -> Result<JsMac, ()> {
    let spanned = parse_js_mac_span(tts)?;
    Ok(JsMac {
        args: spanned
            .args
            .into_iter()
            .map(|arg| {
                match arg {
                    SpanJsMacArg::Ref(refs, derefs, span) => {
                        JsMacArg::Ref(refs, derefs, source_map.source_text(span).unwrap().to_string())
                    }
                    SpanJsMacArg::Primitive(derefs, span, t) => {
                        JsMacArg::Primitive(derefs, source_map.source_text(span).unwrap().to_string(), t)
                    }
                }
            })
            .collect(),
        ret: spanned.ret,
        body: spanned.body,
    })
}

pub fn parse_js_mac_string_source(tts: &[TokenTree], string_source: &str) -> Result<JsMac, ()> {
    let spanned = parse_js_mac_span(tts)?;
    Ok(JsMac {
        args: spanned
            .args
            .into_iter()
            .map(|arg| {
                match arg {
                    SpanJsMacArg::Ref(refs, derefs, span) => {
                        JsMacArg::Ref(refs, derefs, string_source[span.lo..span.hi].to_string())
                    }
                    SpanJsMacArg::Primitive(derefs, span, t) => {
                        JsMacArg::Primitive(derefs, string_source[span.lo..span.hi].to_string(), t)
                    }
                }
            })
            .collect(),
        ret: spanned.ret,
        body: spanned.body,
    })
}