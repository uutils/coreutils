/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Roman Gafiyatullin <r.gafiyatullin@me.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

//!
//! Here we employ shunting-yard algorithm for building AST from tokens according to operators' precedence and associativeness.
//! * https://en.wikipedia.org/wiki/Shunting-yard_algorithm
//!

use std::io::{Write};
use tokens::{Token, OpType};

#[derive(Clone)]
pub enum ASTNode {
    Str{ value: String },
    Int{ value: i64 },
    Op{ operation: OpType, left: Box<ASTNode>, right: Box<ASTNode> }
}

impl ASTNode {
    pub fn bool_value( self ) -> bool {
        if self.int_value() == 0 { false }
        else { true }
    }
    pub fn int_value( self ) -> i64 {
        match self {
            ASTNode::Int{ value: i } => i,
            ASTNode::Str{ value: s } => crash!(2, "not a decimal number: '{}'", s),
            ASTNode::Op{ operation: op_type, left: left_operand, right: right_operand } => {
                match op_type {
                    OpType::Plus => left_operand.int_value() + right_operand.int_value(),
                    OpType::Minus => left_operand.int_value() - right_operand.int_value(),
                    OpType::Mult => left_operand.int_value() * right_operand.int_value(),
                    OpType::Div => left_operand.int_value() / right_operand.int_value(),
                    OpType::Mod => left_operand.int_value() % right_operand.int_value(),
                    OpType::Eq => if left_operand.int_value() == right_operand.int_value() { 1 } else { 0 },
                    OpType::NEq => if left_operand.int_value() != right_operand.int_value() { 1 } else { 0 },
                    OpType::Lt => if left_operand.int_value() < right_operand.int_value() { 1 } else { 0 },
                    OpType::LtEq => if left_operand.int_value() <= right_operand.int_value() { 1 } else { 0 },
                    OpType::Gt => if left_operand.int_value() > right_operand.int_value() { 1 } else { 0 },
                    OpType::GtEq => if left_operand.int_value() >= right_operand.int_value() { 1 } else { 0 },
                    OpType::And => if left_operand.bool_value() && right_operand.bool_value()  { 1 } else { 0 },
                    OpType::Or => if left_operand.bool_value() || right_operand.bool_value()  { 1 } else { 0 },

                    _ => crash!(2, "not implemented: '{:?}'", op_type)
                }
            }
        }
    }
    pub fn str_value( self ) -> String {
        match self {
            ASTNode::Str{ value: s } => s,
            ASTNode::Int{ value: i } => i.to_string(),
            _ => self.int_value().to_string()
        }
    }
}

type TokenStack = Vec<Token>;

pub fn tokens_to_ast( tokens: &Vec<Token> ) -> Box<ASTNode> {
    let mut out_stack: TokenStack = Vec::new();
    let mut op_stack: TokenStack = Vec::new();

    for tok in tokens {
        push_token_to_either_stack( tok.clone(), &mut out_stack, &mut op_stack );
    }

    move_rest_of_ops_to_out( &mut out_stack, &mut op_stack );
    assert!( op_stack.is_empty() );

    ast_from_stack( &mut out_stack )
}

fn ast_from_stack( out_stack: &mut TokenStack ) -> Box<ASTNode> {
    match out_stack.pop() {
        None => crash!(2, "syntax error (incomplete expression)"),

        Some( Token::Str{ value: s } ) => Box::new( ASTNode::Str{ value: s } ),

        Some( Token::Int{ value: s } ) => Box::new( ASTNode::Int{ value: s } ),

        Some( Token::Op{ precedence: _, left_assoc: _, value: op_type } ) => {
            let right = ast_from_stack( out_stack );
            let left = ast_from_stack( out_stack );
            Box::new( ASTNode::Op{ operation: op_type, left: left, right: right } )
        }

        Some( _ ) => panic!("Parenthesis on out_stack")
    }
}

fn push_token_to_either_stack( tok: Token, out_stack: &mut TokenStack, op_stack: &mut TokenStack ) {
    match tok {
        Token::Int{ .. } => out_stack.push( tok ),
        Token::Str{ .. } => out_stack.push( tok ),
        Token::Op{ .. } =>
            if op_stack.is_empty() { op_stack.push( tok ) }
            else { push_op_to_stack( tok, out_stack, op_stack ) },
        Token::ParOpen => op_stack.push( tok ),
        Token::ParClose => move_till_match_paren( out_stack, op_stack )
    }
}


fn push_op_to_stack( tok: Token, out_stack: &mut TokenStack, op_stack: &mut TokenStack ) {
    if let Token::Op{ precedence: prec, left_assoc: la, .. } = tok {
        loop {
            match op_stack.last() {
                None => break,
                Some( &Token::ParOpen ) => {
                    op_stack.push( tok.clone() );
                    break
                },

                Some( &Token::Op{ precedence: prev_prec, .. } ) =>
                    if la && prev_prec >= prec
                    || !la && prev_prec > prec {
                        out_stack.push( op_stack.pop().unwrap() )
                    }
                    else {
                        op_stack.push( tok.clone() );
                        break
                    },

                Some( _ ) => panic!("Non-operator on op_stack")
            }
        }
    }
}

fn move_rest_of_ops_to_out( out_stack: &mut TokenStack, op_stack: &mut TokenStack ) {
    loop {
        match op_stack.pop() {
            None => break,
            Some( Token::ParOpen ) => crash!(2, "syntax error (Mismatched open-parenthesis)"),
            Some( Token::ParClose ) => crash!(2, "syntax error (Mismatched close-parenthesis)"),
            Some( other ) => out_stack.push( other )
        }
    }
}

fn move_till_match_paren( out_stack: &mut TokenStack, op_stack: &mut TokenStack ) {
    loop {
        match op_stack.pop() {
            None => crash!(2, "syntax error (Mismatched close-parenthesis)"),
            Some( Token::ParOpen ) => break,
            Some( other ) => out_stack.push( other )
        }
    }
}
