//* This file is part of the uutils coreutils package.
//*
//* (c) Roman Gafiyatullin <r.gafiyatullin@me.com>
//*
//* For the full copyright and license information, please view the LICENSE
//* file that was distributed with this source code.

//!
//! Here we employ shunting-yard algorithm for building AST from tokens according to operators' precedence and associative-ness.
//! * `<https://en.wikipedia.org/wiki/Shunting-yard_algorithm>`
//!

// spell-checker:ignore (ToDO) binop binops ints paren prec

use num_bigint::BigInt;
use num_traits::{One, Zero};
use onig::{Regex, RegexOptions, Syntax};

use crate::tokens::Token;

type TokenStack = Vec<(usize, Token)>;
pub type OperandsList = Vec<Box<AstNode>>;

#[derive(Debug)]
pub enum AstNode {
    Leaf {
        token_idx: usize,
        value: String,
    },
    Node {
        token_idx: usize,
        op_type: String,
        operands: OperandsList,
    },
}
impl AstNode {
    fn debug_dump(&self) {
        self.debug_dump_impl(1);
    }
    fn debug_dump_impl(&self, depth: usize) {
        for _ in 0..depth {
            print!("\t",);
        }
        match self {
            AstNode::Leaf { token_idx, value } => println!(
                "Leaf( {} ) at #{} ( evaluate -> {:?} )",
                value,
                token_idx,
                self.evaluate()
            ),
            AstNode::Node {
                token_idx,
                op_type,
                operands,
            } => {
                println!(
                    "Node( {} ) at #{} (evaluate -> {:?})",
                    op_type,
                    token_idx,
                    self.evaluate()
                );
                for operand in operands {
                    operand.debug_dump_impl(depth + 1);
                }
            }
        }
    }

    fn new_node(token_idx: usize, op_type: &str, operands: OperandsList) -> Box<AstNode> {
        Box::new(AstNode::Node {
            token_idx,
            op_type: op_type.into(),
            operands,
        })
    }
    fn new_leaf(token_idx: usize, value: &str) -> Box<AstNode> {
        Box::new(AstNode::Leaf {
            token_idx,
            value: value.into(),
        })
    }
    pub fn evaluate(&self) -> Result<String, String> {
        match self {
            AstNode::Leaf { value, .. } => Ok(value.clone()),
            AstNode::Node { op_type, .. } => match self.operand_values() {
                Err(reason) => Err(reason),
                Ok(operand_values) => match op_type.as_ref() {
                    "+" => {
                        infix_operator_two_ints(|a: BigInt, b: BigInt| Ok(a + b), &operand_values)
                    }
                    "-" => {
                        infix_operator_two_ints(|a: BigInt, b: BigInt| Ok(a - b), &operand_values)
                    }
                    "*" => {
                        infix_operator_two_ints(|a: BigInt, b: BigInt| Ok(a * b), &operand_values)
                    }
                    "/" => infix_operator_two_ints(
                        |a: BigInt, b: BigInt| {
                            if b.is_zero() {
                                Err("division by zero".to_owned())
                            } else {
                                Ok(a / b)
                            }
                        },
                        &operand_values,
                    ),
                    "%" => infix_operator_two_ints(
                        |a: BigInt, b: BigInt| {
                            if b.is_zero() {
                                Err("division by zero".to_owned())
                            } else {
                                Ok(a % b)
                            }
                        },
                        &operand_values,
                    ),
                    "=" => infix_operator_two_ints_or_two_strings(
                        |a: BigInt, b: BigInt| Ok(bool_as_int(a == b)),
                        |a: &String, b: &String| Ok(bool_as_string(a == b)),
                        &operand_values,
                    ),
                    "!=" => infix_operator_two_ints_or_two_strings(
                        |a: BigInt, b: BigInt| Ok(bool_as_int(a != b)),
                        |a: &String, b: &String| Ok(bool_as_string(a != b)),
                        &operand_values,
                    ),
                    "<" => infix_operator_two_ints_or_two_strings(
                        |a: BigInt, b: BigInt| Ok(bool_as_int(a < b)),
                        |a: &String, b: &String| Ok(bool_as_string(a < b)),
                        &operand_values,
                    ),
                    ">" => infix_operator_two_ints_or_two_strings(
                        |a: BigInt, b: BigInt| Ok(bool_as_int(a > b)),
                        |a: &String, b: &String| Ok(bool_as_string(a > b)),
                        &operand_values,
                    ),
                    "<=" => infix_operator_two_ints_or_two_strings(
                        |a: BigInt, b: BigInt| Ok(bool_as_int(a <= b)),
                        |a: &String, b: &String| Ok(bool_as_string(a <= b)),
                        &operand_values,
                    ),
                    ">=" => infix_operator_two_ints_or_two_strings(
                        |a: BigInt, b: BigInt| Ok(bool_as_int(a >= b)),
                        |a: &String, b: &String| Ok(bool_as_string(a >= b)),
                        &operand_values,
                    ),
                    "|" => Ok(infix_operator_or(&operand_values)),
                    "&" => Ok(infix_operator_and(&operand_values)),
                    ":" | "match" => operator_match(&operand_values),
                    "length" => Ok(prefix_operator_length(&operand_values)),
                    "index" => Ok(prefix_operator_index(&operand_values)),
                    "substr" => Ok(prefix_operator_substr(&operand_values)),

                    _ => Err(format!("operation not implemented: {}", op_type)),
                },
            },
        }
    }
    pub fn operand_values(&self) -> Result<Vec<String>, String> {
        if let AstNode::Node { operands, .. } = self {
            let mut out = Vec::with_capacity(operands.len());
            for operand in operands {
                let value = operand.evaluate()?;
                out.push(value);
            }
            Ok(out)
        } else {
            panic!("Invoked .operand_values(&self) not with ASTNode::Node")
        }
    }
}

pub fn tokens_to_ast(
    maybe_tokens: Result<Vec<(usize, Token)>, String>,
) -> Result<Box<AstNode>, String> {
    maybe_tokens.and_then(|tokens| {
        let mut out_stack: TokenStack = Vec::new();
        let mut op_stack: TokenStack = Vec::new();

        for (token_idx, token) in tokens {
            push_token_to_either_stack(token_idx, &token, &mut out_stack, &mut op_stack)?;
        }
        move_rest_of_ops_to_out(&mut out_stack, &mut op_stack)?;
        assert!(op_stack.is_empty());

        maybe_dump_rpn(&out_stack);
        let result = ast_from_rpn(&mut out_stack);
        if !out_stack.is_empty() {
            Err(
                "syntax error (first RPN token does not represent the root of the expression AST)"
                    .to_owned(),
            )
        } else {
            maybe_dump_ast(&result);
            result
        }
    })
}

fn maybe_dump_ast(result: &Result<Box<AstNode>, String>) {
    use std::env;
    if let Ok(debug_var) = env::var("EXPR_DEBUG_AST") {
        if debug_var == "1" {
            println!("EXPR_DEBUG_AST");
            match result {
                Ok(ast) => ast.debug_dump(),
                Err(reason) => println!("\terr: {:?}", reason),
            }
        }
    }
}

#[allow(clippy::ptr_arg)]
fn maybe_dump_rpn(rpn: &TokenStack) {
    use std::env;
    if let Ok(debug_var) = env::var("EXPR_DEBUG_RPN") {
        if debug_var == "1" {
            println!("EXPR_DEBUG_RPN");
            for token in rpn {
                println!("\t{:?}", token);
            }
        }
    }
}

fn ast_from_rpn(rpn: &mut TokenStack) -> Result<Box<AstNode>, String> {
    match rpn.pop() {
        None => Err("syntax error (premature end of expression)".to_owned()),

        Some((token_idx, Token::Value { value })) => Ok(AstNode::new_leaf(token_idx, &value)),

        Some((token_idx, Token::InfixOp { value, .. })) => {
            maybe_ast_node(token_idx, &value, 2, rpn)
        }

        Some((token_idx, Token::PrefixOp { value, arity })) => {
            maybe_ast_node(token_idx, &value, arity, rpn)
        }

        Some((token_idx, unexpected_token)) => {
            panic!("unexpected token at #{} {:?}", token_idx, unexpected_token)
        }
    }
}
fn maybe_ast_node(
    token_idx: usize,
    op_type: &str,
    arity: usize,
    rpn: &mut TokenStack,
) -> Result<Box<AstNode>, String> {
    let mut operands = Vec::with_capacity(arity);
    for _ in 0..arity {
        let operand = ast_from_rpn(rpn)?;
        operands.push(operand);
    }
    operands.reverse();
    Ok(AstNode::new_node(token_idx, op_type, operands))
}

fn move_rest_of_ops_to_out(
    out_stack: &mut TokenStack,
    op_stack: &mut TokenStack,
) -> Result<(), String> {
    loop {
        match op_stack.pop() {
            None => return Ok(()),
            Some((token_idx, Token::ParOpen)) => {
                return Err(format!(
                    "syntax error (Mismatched open-parenthesis at #{})",
                    token_idx
                ))
            }
            Some((token_idx, Token::ParClose)) => {
                return Err(format!(
                    "syntax error (Mismatched close-parenthesis at #{})",
                    token_idx
                ))
            }
            Some(other) => out_stack.push(other),
        }
    }
}

fn push_token_to_either_stack(
    token_idx: usize,
    token: &Token,
    out_stack: &mut TokenStack,
    op_stack: &mut TokenStack,
) -> Result<(), String> {
    let result = match token {
        Token::Value { .. } => {
            out_stack.push((token_idx, token.clone()));
            Ok(())
        }

        Token::InfixOp { .. } => {
            if op_stack.is_empty() {
                op_stack.push((token_idx, token.clone()));
                Ok(())
            } else {
                push_op_to_stack(token_idx, token, out_stack, op_stack)
            }
        }

        Token::PrefixOp { .. } => {
            op_stack.push((token_idx, token.clone()));
            Ok(())
        }

        Token::ParOpen => {
            op_stack.push((token_idx, token.clone()));
            Ok(())
        }

        Token::ParClose => move_till_match_paren(out_stack, op_stack),
    };
    maybe_dump_shunting_yard_step(token_idx, token, out_stack, op_stack, &result);
    result
}

#[allow(clippy::ptr_arg)]
fn maybe_dump_shunting_yard_step(
    token_idx: usize,
    token: &Token,
    out_stack: &TokenStack,
    op_stack: &TokenStack,
    result: &Result<(), String>,
) {
    use std::env;
    if let Ok(debug_var) = env::var("EXPR_DEBUG_SYA_STEP") {
        if debug_var == "1" {
            println!("EXPR_DEBUG_SYA_STEP");
            println!("\t{} => {:?}", token_idx, token);
            println!("\t\tout: {:?}", out_stack);
            println!("\t\top : {:?}", op_stack);
            println!("\t\tresult: {:?}", result);
        }
    }
}

fn push_op_to_stack(
    token_idx: usize,
    token: &Token,
    out_stack: &mut TokenStack,
    op_stack: &mut TokenStack,
) -> Result<(), String> {
    if let Token::InfixOp {
        precedence: prec,
        left_assoc: la,
        ..
    } = *token
    {
        loop {
            match op_stack.last() {
                None => {
                    op_stack.push((token_idx, token.clone()));
                    return Ok(());
                }

                Some(&(_, Token::ParOpen)) => {
                    op_stack.push((token_idx, token.clone()));
                    return Ok(());
                }

                Some(&(
                    _,
                    Token::InfixOp {
                        precedence: prev_prec,
                        ..
                    },
                )) => {
                    if la && prev_prec >= prec || !la && prev_prec > prec {
                        out_stack.push(op_stack.pop().unwrap())
                    } else {
                        op_stack.push((token_idx, token.clone()));
                        return Ok(());
                    }
                }

                Some(&(_, Token::PrefixOp { .. })) => {
                    op_stack.push((token_idx, token.clone()));
                    return Ok(());
                }

                Some(_) => panic!("Non-operator on op_stack"),
            }
        }
    } else {
        panic!("Expected infix-op")
    }
}

fn move_till_match_paren(
    out_stack: &mut TokenStack,
    op_stack: &mut TokenStack,
) -> Result<(), String> {
    loop {
        let op = op_stack
            .pop()
            .ok_or_else(|| "syntax error (Mismatched close-parenthesis)".to_string())?;
        match op {
            (_, Token::ParOpen) => return Ok(()),
            other => out_stack.push(other),
        }
    }
}

fn infix_operator_two_ints<F>(f: F, values: &[String]) -> Result<String, String>
where
    F: Fn(BigInt, BigInt) -> Result<BigInt, String>,
{
    assert!(values.len() == 2);
    if let Ok(left) = values[0].parse::<BigInt>() {
        if let Ok(right) = values[1].parse::<BigInt>() {
            return f(left, right).map(|big_int| big_int.to_string());
        }
    }
    Err("Expected an integer operand".to_string())
}

fn infix_operator_two_ints_or_two_strings<FI, FS>(
    fi: FI,
    fs: FS,
    values: &[String],
) -> Result<String, String>
where
    FI: Fn(BigInt, BigInt) -> Result<u8, String>,
    FS: Fn(&String, &String) -> Result<String, String>,
{
    assert!(values.len() == 2);
    if let (Some(a_int), Some(b_int)) = (
        values[0].parse::<BigInt>().ok(),
        values[1].parse::<BigInt>().ok(),
    ) {
        match fi(a_int, b_int) {
            Ok(result) => Ok(result.to_string()),
            Err(reason) => Err(reason),
        }
    } else {
        fs(&values[0], &values[1])
    }
}

fn infix_operator_or(values: &[String]) -> String {
    assert!(values.len() == 2);
    if value_as_bool(&values[0]) {
        values[0].clone()
    } else {
        values[1].clone()
    }
}

fn infix_operator_and(values: &[String]) -> String {
    if value_as_bool(&values[0]) && value_as_bool(&values[1]) {
        values[0].clone()
    } else {
        0.to_string()
    }
}

fn operator_match(values: &[String]) -> Result<String, String> {
    assert!(values.len() == 2);
    let re = Regex::with_options(&values[1], RegexOptions::REGEX_OPTION_NONE, Syntax::grep())
        .map_err(|err| err.description().to_string())?;
    Ok(if re.captures_len() > 0 {
        re.captures(&values[0])
            .map(|captures| captures.at(1).unwrap())
            .unwrap_or("")
            .to_string()
    } else {
        re.find(&values[0])
            .map_or("0".to_string(), |(start, end)| (end - start).to_string())
    })
}

fn prefix_operator_length(values: &[String]) -> String {
    assert!(values.len() == 1);
    values[0].len().to_string()
}

fn prefix_operator_index(values: &[String]) -> String {
    assert!(values.len() == 2);
    let haystack = &values[0];
    let needles = &values[1];

    for (current_idx, ch_h) in haystack.chars().enumerate() {
        for ch_n in needles.chars() {
            if ch_n == ch_h {
                return current_idx.to_string();
            }
        }
    }
    "0".to_string()
}

fn prefix_operator_substr(values: &[String]) -> String {
    assert!(values.len() == 3);
    let subj = &values[0];
    let idx = match values[1]
        .parse::<usize>()
        .ok()
        .and_then(|v| v.checked_sub(1))
    {
        Some(i) => i,
        None => return String::new(),
    };
    let len = match values[2].parse::<usize>() {
        Ok(i) => i,
        Err(_) => return String::new(),
    };

    subj.chars().skip(idx).take(len).collect()
}

fn bool_as_int(b: bool) -> u8 {
    if b {
        1
    } else {
        0
    }
}
fn bool_as_string(b: bool) -> String {
    if b {
        "1".to_string()
    } else {
        "0".to_string()
    }
}
fn value_as_bool(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    match s.parse::<BigInt>() {
        Ok(n) => n.is_one(),
        Err(_) => true,
    }
}
