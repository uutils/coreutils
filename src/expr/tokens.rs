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
//! * binary operators with 6 priorities.
//!
//! According to the man-page of expr we have expression split into tokens (each token -- separate CLI-argument).
//! Hence all we need is to map the strings into the Token structures.
//!

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpType {
    Colon,
    Mult,
    Div,
    Mod,
    Plus,
    Minus,
    Eq,
    Gt,
    GtEq,
    Lt,
    LtEq,
    NEq,
    And,
    Or,
    Cap
}

#[derive(Clone, PartialEq)]
pub enum Token {
    Int{ value: i64 },
    Str{ value: String },
    ParOpen,
    ParClose,
    Op{
            precedence: u8,
            left_assoc: bool,
            value: OpType
        }
}

pub fn string_vec_to_tokens( args: &Vec<String> ) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity( args.len() );
    for s in args {
        match s.as_ref() {
            "(" => out.push( Token::ParOpen ),
            ")" => out.push( Token::ParClose ),

            // The following one is added just to prove right-associativeness working
            "^" => out.push( Token::Op{ left_assoc: false, precedence: 7, value: OpType::Cap } ),

            ":" => out.push( Token::Op{ left_assoc: true, precedence: 6, value: OpType::Colon } ),

            "*" => out.push( Token::Op{ left_assoc: true, precedence: 5, value: OpType::Mult } ),
            "/" => out.push( Token::Op{ left_assoc: true, precedence: 5, value: OpType::Div } ),
            "%" => out.push( Token::Op{ left_assoc: true, precedence: 5, value: OpType::Mod } ),

            "+" => out.push( Token::Op{ left_assoc: true, precedence: 4, value: OpType::Plus } ),
            "-" => out.push( Token::Op{ left_assoc: true, precedence: 4, value: OpType::Minus } ),

            "=" => out.push( Token::Op{ left_assoc: true, precedence: 3, value: OpType::Eq } ),
            ">" => out.push( Token::Op{ left_assoc: true, precedence: 3, value: OpType::Gt } ),
            ">=" => out.push( Token::Op{ left_assoc: true, precedence: 3, value: OpType::GtEq } ),
            "<" => out.push( Token::Op{ left_assoc: true, precedence: 3, value: OpType::Lt } ),
            "<=" => out.push( Token::Op{ left_assoc: true, precedence: 3, value: OpType::LtEq } ),
            "!=" => out.push( Token::Op{ left_assoc: true, precedence: 3, value: OpType::NEq } ),

            "&" => out.push( Token::Op{ left_assoc: true, precedence: 2, value: OpType::And } ),

            "|" => out.push( Token::Op{ left_assoc: true, precedence: 1, value: OpType::Or } ),

            s => match s.parse::<i64>() {
                    Ok( i ) => out.push( Token::Int{ value: i } ),
                    Err( _ ) =>
                        // panic!("Ouch! A string")
                        out.push( Token::Str{ value: s.to_string() } )
                },
        }
    }
    out
}
