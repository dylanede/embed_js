extern crate cpp_synmap;
extern crate cpp_syn;
#[macro_use]
extern crate quote;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use std::borrow::Cow;

use cpp_synmap::SourceMap;
use cpp_syn::{TokenTree, Delimited, DelimToken, Token, Span};

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
pub struct JsMac<'a> {
    pub args: Vec<(Cow<'a, str>, WasmPrimitiveType)>,
    pub ret: Option<WasmPrimitiveType>,
    pub body: Option<String>,
}

struct SpanJsMac {
    args: Vec<(Span, WasmPrimitiveType)>,
    ret: Option<WasmPrimitiveType>,
    body: Option<String>,
}

fn parse_js_mac_span(tts: &[TokenTree]) -> Result<SpanJsMac, ()> {
    let mut iter = tts.iter().peekable();
    let (args, ret) = match iter.peek() {
        Some(&&TokenTree::Delimited(Delimited { delim, ref tts }, _)) => {
            match delim {
                DelimToken::Bracket => {
                    iter.next(); // consume
                    let mut args = Vec::new();
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
                            let name = match iter.next() {
                                Some(&TokenTree::Token(Token::Ident(_), span)) => span,
                                None => break,
                                _ => return Err(()),
                            };
                            match iter.next() {
                                Some(&TokenTree::Token(Token::Ident(ref ident), _))
                                if ident.as_ref() == "as" => {}
                                _ => return Err(()),
                            }
                            args.push((name, parse_wasm_primitive_type(&mut iter)?));
                        }
                    }
                    let ret = if let Some(&&TokenTree::Token(Token::RArrow, _)) = iter.peek() {
                        iter.next();
                        Some(parse_wasm_primitive_type(&mut iter)?)
                    } else {
                        None
                    };
                    (args, ret)
                }
                DelimToken::Brace => (vec![], None), // no params or return,
                _ => return Err(()),
            }
        }
        _ => return Err(()),
    };

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

pub fn parse_js_mac_source_map<'a>(tts: &[TokenTree], source_map: &'a SourceMap) -> Result<JsMac<'a>, ()> {
    let spanned = parse_js_mac_span(tts)?;
    Ok(JsMac {
        args: spanned
            .args
            .iter()
            .map(|&(span, t)| {
                (Cow::from(source_map.source_text(span).unwrap()), t)
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
            .iter()
            .map(|&(span, t)| {
                (Cow::from(&string_source[span.lo..span.hi]), t)
            })
            .collect(),
        ret: spanned.ret,
        body: spanned.body,
    })
}