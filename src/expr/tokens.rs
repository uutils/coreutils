/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Roman Gafiyatullin <r.gafiyatullin@me.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

//!
//! The following tokens are present in the expr grammar:
//! * integer literal;
//! * string literal;
//! * infix binary operators;
//! * prefix operators.
//!
//! According to the man-page of expr we have expression split into tokens (each token -- separate CLI-argument).
//! Hence all we need is to map the strings into the Token structures, except for some ugly fiddling with +-escaping.
//!

#[derive(Debug, Clone)]
pub enum Token {
    Value {
        value: String,
    },

    ParOpen,
    ParClose,

    InfixOp {
        precedence: u8,
        left_assoc: bool,
        value: String,
    },

    PrefixOp {
        arity: usize,
        value: String,
    },
}
impl Token {
    fn new_infix_op(v: &String, left_assoc: bool, precedence: u8) -> Self {
        Token::InfixOp {
            left_assoc: left_assoc,
            precedence: precedence,
            value: v.clone(),
        }
    }
    fn new_value(v: &String) -> Self {
        Token::Value { value: v.clone() }
    }

    fn is_infix_plus(&self) -> bool {
        match *self {
            Token::InfixOp { ref value, .. } => value == "+",
            _ => false,
        }
    }
    fn is_a_number(&self) -> bool {
        match *self {
            Token::Value { ref value, .. } => match value.parse::<i64>() {
                Ok(_) => true,
                Err(_) => false,
            },
            _ => false,
        }
    }
    fn is_a_close_paren(&self) -> bool {
        match *self {
            Token::ParClose => true,
            _ => false,
        }
    }
}

pub fn strings_to_tokens(strings: &[String]) -> Result<Vec<(usize, Token)>, String> {
    let mut tokens_acc = Vec::with_capacity(strings.len());
    let mut tok_idx = 1;

    for s in strings {
        let token_if_not_escaped = match s.as_ref() {
            "(" => Token::ParOpen,
            ")" => Token::ParClose,

            "^" => Token::new_infix_op(&s, false, 7),

            ":" => Token::new_infix_op(&s, true, 6),

            "*" => Token::new_infix_op(&s, true, 5),
            "/" => Token::new_infix_op(&s, true, 5),
            "%" => Token::new_infix_op(&s, true, 5),

            "+" => Token::new_infix_op(&s, true, 4),
            "-" => Token::new_infix_op(&s, true, 4),

            "=" => Token::new_infix_op(&s, true, 3),
            "!=" => Token::new_infix_op(&s, true, 3),
            "<" => Token::new_infix_op(&s, true, 3),
            ">" => Token::new_infix_op(&s, true, 3),
            "<=" => Token::new_infix_op(&s, true, 3),
            ">=" => Token::new_infix_op(&s, true, 3),

            "&" => Token::new_infix_op(&s, true, 2),

            "|" => Token::new_infix_op(&s, true, 1),

            "match" => Token::PrefixOp {
                arity: 2,
                value: s.clone(),
            },
            "substr" => Token::PrefixOp {
                arity: 3,
                value: s.clone(),
            },
            "index" => Token::PrefixOp {
                arity: 2,
                value: s.clone(),
            },
            "length" => Token::PrefixOp {
                arity: 1,
                value: s.clone(),
            },

            _ => Token::new_value(&s),
        };
        push_token_if_not_escaped(&mut tokens_acc, tok_idx, token_if_not_escaped, &s);
        tok_idx += 1;
    }
    maybe_dump_tokens_acc(&tokens_acc);

    Ok(tokens_acc)
}

fn maybe_dump_tokens_acc(tokens_acc: &[(usize, Token)]) {
    use std::env;

    if let Ok(debug_var) = env::var("EXPR_DEBUG_TOKENS") {
        if debug_var == "1" {
            println!("EXPR_DEBUG_TOKENS");
            for token in tokens_acc {
                println!("\t{:?}", token);
            }
        }
    }
}

fn push_token_if_not_escaped(
    acc: &mut Vec<(usize, Token)>,
    tok_idx: usize,
    token: Token,
    s: &String,
) {
    // Smells heuristics... :(
    let prev_is_plus = match acc.last() {
        None => false,
        Some(ref t) => t.1.is_infix_plus(),
    };
    let should_use_as_escaped = if prev_is_plus && acc.len() >= 2 {
        let pre_prev = &acc[acc.len() - 2];
        !(pre_prev.1.is_a_number() || pre_prev.1.is_a_close_paren())
    } else {
        prev_is_plus
    };

    if should_use_as_escaped {
        acc.pop();
        acc.push((tok_idx, Token::new_value(s)))
    } else {
        acc.push((tok_idx, token))
    }
}
