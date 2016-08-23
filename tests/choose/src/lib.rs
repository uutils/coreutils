#![feature(plugin_registrar, rustc_private)]

extern crate syntax;
extern crate rustc_plugin;

use syntax::ast;
use syntax::ptr::P;
use syntax::codemap::Span;
use syntax::parse::{self, token};
use syntax::tokenstream::TokenTree;
use syntax::ext::base::{ExtCtxt, MacResult, DummyResult, MacEager};
use syntax::ext::build::AstBuilder;
use syntax::errors::FatalError;
use syntax::util::small_vector::SmallVector;
use rustc_plugin::Registry;

use ::std::path::Path;
use ::std::path::PathBuf;

macro_rules! panictry {
    ($e:expr) => ({
        match $e {
            Ok(e) => e,
            Err(mut e) => {
                e.emit();
                panic!(FatalError);
            }
        }
    })
}

pub fn expand_include<'cx>(cx: &'cx mut ExtCtxt, sp: Span, file: &Path) -> Vec<P<ast::Item>> {
    let mut p = parse::new_sub_parser_from_file(cx.parse_sess(), cx.cfg(), file, true, None, sp);
    let mut ret = vec![];
    while p.token != token::Eof {
        match panictry!(p.parse_item()) {
            Some(item) => ret.push(item),
            None => {
                panic!(p.diagnostic().span_fatal(p.span,
                                                 &format!("expected item, found `{}`", p.this_token_to_string())))
            }
        }
    }
    ret
}

fn intern(s: &str) -> token::InternedString {
    token::intern_and_get_ident(s)
}

fn choose(cx: &mut ExtCtxt, sp: Span, args: &[TokenTree]) -> Box<MacResult + 'static> {
    let mut parser = cx.new_parser_from_tts(args);
    let mut test_mods = SmallVector::zero();
    let cfg_str = intern("cfg");
    let feat_str = intern("feature");
    while !parser.eat(&token::Eof) {
        if let Ok(s) = parser.parse_ident() {
            let unix_only;
            match s.to_string().as_str() {
                "unix" => unix_only = true,
                "generic" => unix_only = false,
                _ => {
                    cx.span_err(sp, "only `unix` and `generic` are supported now");
                    return DummyResult::any(sp);
                }
            }
            parser.eat(&token::FatArrow);
            parser.eat(&token::OpenDelim(token::Brace));
            while !parser.eat(&token::CloseDelim(token::Brace)) {
                match parser.parse_ident() {
                    Ok(s) => {
                        let mod_name = s.to_string();
                        let mut attrs = vec![cx.attribute(sp,
                                                          cx.meta_list(sp,
                                                                       cfg_str.clone(),
                                                                       vec![cx.meta_name_value(sp,
                                                                                               feat_str.clone(),
                                                                                               ast::LitKind::Str(intern(mod_name.trim_left_matches("test_")), ast::StrStyle::Cooked))]))];

                        if unix_only {
                            attrs.push(cx.attribute(sp,
                                                    cx.meta_list(sp,
                                                                 cfg_str.clone(),
                                                                 vec![cx.meta_word(sp, intern("unix"))])));
                        }

                        let mut mod_path = PathBuf::from(&cx.codemap().span_to_filename(sp));
                        mod_path.set_file_name(mod_name.as_str());
                        mod_path.set_extension("rs");
                        test_mods.push(P(ast::Item {
                            ident: cx.ident_of(mod_name.as_str()),
                            attrs: attrs,
                            id: ast::DUMMY_NODE_ID,
                            node: ast::ItemKind::Mod(ast::Mod {
                                inner: sp,
                                items: expand_include(cx, sp, &mod_path),
                            }),
                            vis: ast::Visibility::Inherited,
                            span: sp,
                        }));
                    }
                    _ => {
                        cx.span_err(sp, "expect a single identifier");
                        return DummyResult::any(sp);
                    }
                }
                parser.eat(&token::Semi);
                parser.eat(&token::Comma);
            }
        } else {
            cx.span_err(sp, "expect a single identifier");
            return DummyResult::any(sp);
        }
    }
    MacEager::items(test_mods)
}

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("choose", choose);
}
