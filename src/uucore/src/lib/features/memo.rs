//! Memo runner of printf
//! Takes a format string and arguments
//! 1. tokenize format string into tokens, consuming
//! any subst. arguments along the way.
//! 2. feeds remaining arguments into function
//! that prints tokens.

use crate::display::Quotable;
use crate::error::UResult;
use crate::features::tokenize::sub::Sub;
use crate::features::tokenize::token::{Token, Tokenizer};
use crate::features::tokenize::unescaped_text::UnescapedText;
use crate::show_warning;
use itertools::put_back_n;
use std::iter::Peekable;
use std::slice::Iter;

pub struct Memo {
    tokens: Vec<Box<dyn Token>>,
}

fn warn_excess_args(first_arg: &str) {
    show_warning!(
        "ignoring excess arguments, starting with {}",
        first_arg.quote()
    );
}

impl Memo {
    pub fn new(pf_string: &str, pf_args_it: &mut Peekable<Iter<String>>) -> UResult<Self> {
        let mut pm = Self { tokens: Vec::new() };
        let mut tmp_token: Option<Box<dyn Token>>;
        let mut it = put_back_n(pf_string.chars());
        let mut has_sub = false;
        loop {
            tmp_token = UnescapedText::from_it(&mut it, pf_args_it)?;
            if let Some(x) = tmp_token {
                pm.tokens.push(x);
            }
            tmp_token = Sub::from_it(&mut it, pf_args_it)?;
            if let Some(x) = tmp_token {
                if !has_sub {
                    has_sub = true;
                }
                pm.tokens.push(x);
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
        Ok(pm)
    }
    pub fn apply(&self, pf_args_it: &mut Peekable<Iter<String>>) {
        for tkn in &self.tokens {
            tkn.print(pf_args_it);
        }
    }
    pub fn run_all(pf_string: &str, pf_args: &[String]) -> UResult<()> {
        let mut arg_it = pf_args.iter().peekable();
        let pm = Self::new(pf_string, &mut arg_it)?;
        loop {
            if arg_it.peek().is_none() {
                return Ok(());
            }
            pm.apply(&mut arg_it);
        }
    }
}
