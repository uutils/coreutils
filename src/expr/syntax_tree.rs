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

use tokens::{Token};
use onig::{Regex, Syntax, REGEX_OPTION_NONE};

type TokenStack = Vec<(usize, Token)>;
pub type OperandsList = Vec< Box<ASTNode> >;

#[derive(Debug)]
pub enum ASTNode {
    Leaf { token_idx: usize, value: String },
    Node { token_idx: usize, op_type: String, operands: OperandsList }
}
impl ASTNode {
    fn debug_dump( &self ) {
        self.debug_dump_impl( 1 );
    }
    fn debug_dump_impl( &self, depth: usize ) {
        for _ in 0..depth {
            print!("\t", );
        }
        match *self {
            ASTNode::Leaf{ ref token_idx, ref value } => println!("Leaf( {} ) at #{} ( evaluate -> {:?} )", value, token_idx, self.evaluate()),
            ASTNode::Node{ ref token_idx, ref op_type, ref operands } => {
                println!("Node( {} ) at #{} (evaluate -> {:?})", op_type, token_idx, self.evaluate());
                for operand in operands {
                    operand.debug_dump_impl( depth + 1 );
                }
            }
        }
    }

    fn new_node( token_idx: usize, op_type: &String, operands: OperandsList ) -> Box<ASTNode> {
        Box::new( ASTNode::Node{
                token_idx: token_idx,
                op_type: op_type.clone(),
                operands: operands
            } )
    }
    fn new_leaf( token_idx: usize, value: &String ) -> Box<ASTNode> {
        Box::new( ASTNode::Leaf{ token_idx: token_idx, value: value.clone() } )
    }
    pub fn evaluate( &self ) -> Result<String, String> {
        match *self {
            ASTNode::Leaf{ ref value, .. } => Ok( value.clone() ),
            ASTNode::Node{ ref op_type, .. } =>
                match self.operand_values() {
                    Err( reason ) => Err( reason ),
                    Ok( operand_values ) =>
                        match op_type.as_ref() {
                            "+" => infix_operator_two_ints( |a: i64, b: i64| Ok( a + b ), &operand_values ),
                            "-" => infix_operator_two_ints( |a: i64, b: i64| Ok( a - b ), &operand_values ),
                            "*" => infix_operator_two_ints( |a: i64, b: i64| Ok( a * b ), &operand_values ),
                            "/" => infix_operator_two_ints(
                                |a: i64, b: i64|
                                    if b == 0 { Err("division by zero".to_owned()) }
                                    else { Ok( a / b ) },
                                &operand_values ),
                            "%" => infix_operator_two_ints(
                                |a: i64, b: i64|
                                    if b == 0 { Err("division by zero".to_owned()) }
                                    else { Ok( a % b ) },
                                &operand_values ),

                            "=" => infix_operator_two_ints_or_two_strings(
                                    |a: i64, b: i64| Ok( bool_as_int(a == b) ),
                                    |a: &String, b: &String| Ok( bool_as_string(a == b) ),
                                    &operand_values
                                ),
                            "!=" => infix_operator_two_ints_or_two_strings(
                                    |a: i64, b: i64| Ok( bool_as_int(a != b) ),
                                    |a: &String, b: &String| Ok( bool_as_string(a != b) ),
                                    &operand_values
                                ),
                            "<" => infix_operator_two_ints_or_two_strings(
                                    |a: i64, b: i64| Ok( bool_as_int(a < b) ),
                                    |a: &String, b: &String| Ok( bool_as_string(a < b) ),
                                    &operand_values
                                ),
                            ">" => infix_operator_two_ints_or_two_strings(
                                    |a: i64, b: i64| Ok( bool_as_int(a > b) ),
                                    |a: &String, b: &String| Ok( bool_as_string(a > b) ),
                                    &operand_values
                                ),
                            "<=" => infix_operator_two_ints_or_two_strings(
                                    |a: i64, b: i64| Ok( bool_as_int(a <= b) ),
                                    |a: &String, b: &String| Ok( bool_as_string(a <= b) ),
                                    &operand_values
                                ),
                            ">=" => infix_operator_two_ints_or_two_strings(
                                    |a: i64, b: i64| Ok( bool_as_int(a >= b) ),
                                    |a: &String, b: &String| Ok( bool_as_string(a >= b) ),
                                    &operand_values
                                ),
                            "|" => infix_operator_or(&operand_values),
                            "&" => infix_operator_and(&operand_values),
                            ":" | "match" => operator_match(&operand_values),
                            "length" => prefix_operator_length( &operand_values ),
                            "index" => prefix_operator_index( &operand_values ),
                            "substr" => prefix_operator_substr( &operand_values ),

                            _ => Err(format!("operation not implemented: {}", op_type))
                        }
                }
        }
    }
    pub fn operand_values( &self ) -> Result<Vec<String>, String> {
        if let &ASTNode::Node{ ref operands, .. } = self {
            let mut out = Vec::with_capacity( operands.len() );
            for operand in operands {
                match operand.evaluate() {
                    Ok( value ) => out.push( value ),
                    Err( reason ) => return Err( reason ),
                }
            }
            Ok( out )
        }
        else { panic!("Invoked .operand_values(&self) not with ASTNode::Node") }
    }
}

pub fn tokens_to_ast( maybe_tokens: Result< Vec<(usize, Token)>, String > ) -> Result<Box<ASTNode>, String> {
    if maybe_tokens.is_err() { Err( maybe_tokens.err().unwrap() ) }
    else {
        let tokens = maybe_tokens.ok().unwrap();
        let mut out_stack: TokenStack = Vec::new();
        let mut op_stack: TokenStack = Vec::new();

        for (token_idx, token) in tokens {
            if let Err( reason ) = push_token_to_either_stack( token_idx, &token, &mut out_stack, &mut op_stack ) {
                return Err( reason )
            }
        }
        if let Err( reason ) = move_rest_of_ops_to_out( &mut out_stack, &mut op_stack ) {
            return Err( reason )
        }
        assert!( op_stack.is_empty() );

        maybe_dump_rpn( &out_stack );
        let result = ast_from_rpn( &mut out_stack );
        if !out_stack.is_empty() {
            Err( "syntax error (fist RPN token does not represent expression AST's root)".to_owned() )
        }
        else {
            maybe_dump_ast( &result );
            result
        }
    }
}

fn maybe_dump_ast( result: &Result< Box<ASTNode>, String > ) {
    use std::env;
    if let Ok( debug_var ) = env::var( "EXPR_DEBUG_AST" ) {
        if debug_var == "1" {
            println!("EXPR_DEBUG_AST");
            match *result {
                Ok( ref ast ) => ast.debug_dump(),
                Err( ref reason ) => println!("\terr: {:?}", reason),
           }
        }
    }
}

fn maybe_dump_rpn( rpn: &TokenStack ) {
    use std::env;
    if let Ok( debug_var ) = env::var( "EXPR_DEBUG_RPN" ) {
        if debug_var == "1" {
            println!("EXPR_DEBUG_RPN");
            for token in rpn {
                println!("\t{:?}", token);
            }
        }
    }
}

fn ast_from_rpn( rpn: &mut TokenStack ) -> Result<Box<ASTNode>, String> {
    match rpn.pop() {
        None => Err( "syntax error (premature end of expression)".to_owned() ),

        Some( (token_idx, Token::Value{ value }) ) =>
            Ok( ASTNode::new_leaf( token_idx, &value ) ),

        Some( (token_idx, Token::InfixOp{ value, .. }) ) =>
            maybe_ast_node( token_idx, &value, 2, rpn ),

        Some( (token_idx, Token::PrefixOp{ value, arity }) ) =>
            maybe_ast_node( token_idx, &value, arity, rpn ),

        Some( (token_idx, unexpected_token) ) =>
            panic!("unexpected token at #{} {:?}", token_idx, unexpected_token),
    }
}
fn maybe_ast_node( token_idx: usize, op_type: &String, arity: usize, rpn: &mut TokenStack ) -> Result< Box<ASTNode>, String > {
    let mut operands = Vec::with_capacity( arity );
    for _ in 0..arity {
        match ast_from_rpn( rpn ) {
            Err( reason ) => return Err( reason ),
            Ok( operand ) => operands.push( operand ),
        }
    }
    operands.reverse();
    Ok( ASTNode::new_node( token_idx, op_type, operands ) )
}

fn move_rest_of_ops_to_out( out_stack: &mut TokenStack, op_stack: &mut TokenStack ) -> Result<(), String> {
    loop {
        match op_stack.pop() {
            None => return Ok( () ),
            Some( (token_idx, Token::ParOpen) ) => return Err( format!( "syntax error (Mismatched open-parenthesis at #{})", token_idx ) ),
            Some( (token_idx, Token::ParClose) ) => return Err( format!( "syntax error (Mismatched close-parenthesis at #{})", token_idx ) ),
            Some( other ) => out_stack.push( other )
        }
    }
}

fn push_token_to_either_stack( token_idx: usize, token: &Token, out_stack: &mut TokenStack, op_stack: &mut TokenStack ) -> Result<(), String> {
    let result =
        match *token {
            Token::Value{ .. } => Ok( out_stack.push( (token_idx, token.clone()) ) ),

            Token::InfixOp{ .. } =>
                if op_stack.is_empty() { Ok( op_stack.push( (token_idx, token.clone()) ) ) }
                else { push_op_to_stack( token_idx, token, out_stack, op_stack ) },

            Token::PrefixOp{ .. } => Ok( op_stack.push( (token_idx, token.clone()) ) ),

            Token::ParOpen => Ok( op_stack.push( (token_idx, token.clone()) ) ),

            Token::ParClose => move_till_match_paren( out_stack, op_stack )
        };
    maybe_dump_shunting_yard_step( token_idx, token, out_stack, op_stack, &result );
    result
}

fn maybe_dump_shunting_yard_step( token_idx: usize, token: &Token, out_stack: &TokenStack, op_stack: &TokenStack, result: &Result<(), String> ) {
    use std::env;
    if let Ok( debug_var ) = env::var( "EXPR_DEBUG_SYA_STEP" ) {
        if debug_var == "1" {
            println!("EXPR_DEBUG_SYA_STEP");
            println!("\t{} => {:?}", token_idx, token);
            println!("\t\tout: {:?}", out_stack);
            println!("\t\top : {:?}", op_stack);
            println!("\t\tresult: {:?}", result);
        }
    }
}

fn push_op_to_stack( token_idx: usize, token: &Token, out_stack: &mut TokenStack, op_stack: &mut TokenStack ) -> Result<(), String> {
    if let &Token::InfixOp{ precedence: prec, left_assoc: la, .. } = token {
        loop {
            match op_stack.last() {
                None =>
                    return Ok( op_stack.push( (token_idx, token.clone()) ) ),

                Some( &(_, Token::ParOpen) ) =>
                    return Ok( op_stack.push( (token_idx, token.clone()) ) ),

                Some( &(_, Token::InfixOp{ precedence: prev_prec, .. }) ) =>
                    if la && prev_prec >= prec
                    || !la && prev_prec > prec {
                        out_stack.push( op_stack.pop().unwrap() )
                    }
                    else {
                        return Ok( op_stack.push( (token_idx, token.clone()) ) )
                    },

                Some( &(_, Token::PrefixOp{ .. }) ) =>
                    return Ok( op_stack.push( (token_idx, token.clone()) ) ),

                Some( _ ) => panic!("Non-operator on op_stack")
            }
        }
    }
    else {
        panic!("Expected infix-op")
    }
}

fn move_till_match_paren( out_stack: &mut TokenStack, op_stack: &mut TokenStack ) -> Result<(), String> {
    loop {
        match op_stack.pop() {
            None => return Err( "syntax error (Mismatched close-parenthesis)".to_string() ),
            Some( (_, Token::ParOpen) ) => return Ok( () ),
            Some( other ) => out_stack.push( other )
        }
    }
}


fn infix_operator_two_ints<F>( f: F, values: &Vec<String> ) -> Result<String, String>
    where F : Fn( i64, i64 ) -> Result<i64, String>
{
    assert!( values.len() == 2 );
    if let Some( left ) = values[0].parse::<i64>().ok() {
        if let Some( right ) = values[1].parse::<i64>().ok() {
            return match f( left, right ) {
                Ok(result) => Ok(result.to_string()),
                Err(reason) => Err(reason),
            }
        }
    }
    Err( "Expected an integer operand".to_string() )
}


fn infix_operator_two_ints_or_two_strings<FI, FS>( fi: FI, fs: FS, values: &Vec<String> ) -> Result<String, String>
    where FI : Fn( i64, i64 ) -> Result<i64, String>,
          FS : Fn( &String, &String ) -> Result<String, String>
{
    assert!( values.len() == 2 );
    if let ( Some( a_int ), Some( b_int ) ) =
        (
            values[0].parse::<i64>().ok(),
            values[1].parse::<i64>().ok()
        ) {
            match fi( a_int, b_int ) {
                Ok( result ) => Ok(result.to_string()),
                Err( reason ) => Err(reason)
            }
        }
    else {
        fs( &values[0], &values[1] )
    }
}

fn infix_operator_or( values: &Vec<String> ) -> Result<String, String> {
    assert!(values.len() == 2);
    if value_as_bool(&values[0]) {
        Ok(values[0].clone())
    } else {
        Ok(values[1].clone())
    }
}

fn infix_operator_and( values: &Vec<String> ) -> Result<String, String> {
    if value_as_bool(&values[0]) && value_as_bool(&values[1]) {
        Ok(values[0].clone())
    } else {
        Ok(0.to_string())
    }
}

fn operator_match(values: &Vec<String>) -> Result<String, String> {
    assert!(values.len() == 2);
    let re = match Regex::with_options(&values[1], REGEX_OPTION_NONE, Syntax::grep()) {
        Ok(m) => m,
        Err(err) => return Err(err.description().to_string())
    };
    if re.captures_len() > 0 {
        Ok(match re.captures(&values[0]) {
            Some(captures) => captures.at(1).unwrap().to_string(),
            None => "".to_string()
        })
    } else {
        Ok(match re.find(&values[0]) {
            Some((start, end)) => (end - start).to_string(),
            None => "0".to_string()
        })
    }
}

fn prefix_operator_length( values: &Vec<String> ) -> Result<String, String> {
    assert!( values.len() == 1 );
    Ok( values[0].len().to_string() )
}

fn prefix_operator_index( values: &Vec<String> ) -> Result<String, String> {
    assert!( values.len() == 2 );
    let haystack = &values[0];
    let needles = &values[1];

    let mut current_idx = 0;
    for ch_h in haystack.chars() {
        current_idx += 1;

        for ch_n in needles.chars() {
            if ch_n == ch_h {
                return Ok( current_idx.to_string() )
            }
        }
    }
    Ok( "0".to_string() )
}

fn prefix_operator_substr( values: &Vec<String> ) -> Result<String, String> {
    assert!( values.len() == 3 );
    let subj = &values[0];
    let mut idx = match values[1].parse::<i64>() {
        Ok( i ) => i,
        Err( _ ) => return Err( "expected integer as POS arg to 'substr'".to_string() ),
    };
    let mut len = match values[2].parse::<i64>() {
        Ok( i ) => i,
        Err( _ ) => return Err( "expected integer as LENGTH arg to 'substr'".to_string() ),
    };

    if idx <= 0 || len <= 0 { return Ok( "".to_string() ) }

    let mut out_str = String::new();
    for ch in subj.chars() {
        idx -= 1;
        if idx <= 0 {
            if len <= 0 { break; }
            len -= 1;

            out_str.push( ch );
        }
    }
    Ok( out_str )
}

fn bool_as_int( b: bool ) -> i64 { if b { 1 } else { 0 } }
fn bool_as_string( b: bool ) -> String { if b { "1".to_string() } else { "0".to_string() } }
fn value_as_bool( s: &str ) -> bool {
    if s.len() == 0 {
        return false
    }
    match s.parse::<i64>() {
        Ok(n) => n != 0,
        Err(_) => true,
    }
}
