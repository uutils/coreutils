//! Memo runner of printf
//! Takes a format string and arguments
//! 1. tokenizes format string into tokens, consuming
//! any subst. arguments along the way.
//! 2. feeds remaining arguments into function
//! that prints tokens.

use std::iter::Peekable;
use std::slice::Iter;
use itertools::put_back_n;
use cli;
use tokenize::token::{Token, Tokenizer};
use tokenize::unescaped_text::UnescapedText;
use tokenize::sub::Sub;

pub struct Memo {
    tokens: Vec<Box<Token>>,
}

fn warn_excess_args(first_arg: &str) {
    cli::err_msg(&format!("warning: ignoring excess arguments, starting with '{}'",
                          first_arg));
}

impl Memo {
    pub fn new(pf_string: &String, pf_args_it: &mut Peekable<Iter<String>>) -> Memo {
        let mut pm = Memo { tokens: Vec::new() };
        let mut tmp_token: Option<Box<Token>>;
        let mut it = put_back_n(pf_string.chars());
        let mut has_sub = false;
        loop {
            tmp_token = UnescapedText::from_it(&mut it, pf_args_it);
            match tmp_token {
                Some(x) => pm.tokens.push(x),
                None => {}
            }
            tmp_token = Sub::from_it(&mut it, pf_args_it);
            match tmp_token {
                Some(x) => {
                    if !has_sub {
                        has_sub = true;
                    }
                    pm.tokens.push(x);
                }
                None => {}
            }
            if let Some(x) = it.next() {
                it.put_back(x);
            } else {
                break;
            }
        }
        if !has_sub {
            let mut drain = false;
            if let Some(first_arg) = pf_args_it.peek() {
                warn_excess_args(first_arg);
                drain = true;
            }
            if drain {
                loop {
                    // drain remaining args;
                    if pf_args_it.next().is_none() {
                        break;
                    }
                }
            }
        }
        pm
    }
    pub fn apply(&self, pf_args_it: &mut Peekable<Iter<String>>) {
        for tkn in self.tokens.iter() {
            tkn.print(pf_args_it);
        }
    }
    pub fn run_all(pf_string: &String, pf_args: &[String]) {
        let mut arg_it = pf_args.iter().peekable();
        let pm = Memo::new(pf_string, &mut arg_it);
        loop {
            if arg_it.peek().is_none() {
                break;
            }
            pm.apply(&mut arg_it);
        }
    }
}
