// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (grammar) BOOL_OP INT_OP STRING_OP FILE_OP UNARY_OP lparen rparen nargs

use std::ffi::{OsStr, OsString};

use super::error::{ParseError, ParseErrorKind, ParseResult};
use uucore::display::Quotable;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BinaryOp {
    StrEq,
    StrNe,
    StrLt,
    StrGt,
    IntEq,
    IntNe,
    IntLt,
    IntLe,
    IntGt,
    IntGe,
    FileEf,
    FileNt,
    FileOt,
}

impl BinaryOp {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::StrEq => "=",
            Self::StrNe => "!=",
            Self::StrLt => "<",
            Self::StrGt => ">",
            Self::IntEq => "-eq",
            Self::IntNe => "-ne",
            Self::IntLt => "-lt",
            Self::IntLe => "-le",
            Self::IntGt => "-gt",
            Self::IntGe => "-ge",
            Self::FileEf => "-ef",
            Self::FileNt => "-nt",
            Self::FileOt => "-ot",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnaryOp {
    BlockSpecial,
    CharacterSpecial,
    Directory,
    Exists,
    Regular,
    GroupIdFlag,
    GroupOwns,
    SymLink,
    Sticky,
    ModifiedSinceRead,
    UserOwns,
    Fifo,
    Readable,
    NonEmpty,
    Socket,
    Tty,
    UserIdFlag,
    Writable,
    Executable,
    StrNonEmpty,
    StrEmpty,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Operand<'a> {
    Value(&'a OsStr),
    Length(&'a OsStr),
}

pub(crate) trait Evaluator {
    fn unary(&mut self, op: UnaryOp, arg: &OsStr) -> ParseResult<bool>;
    fn binary(&mut self, op: BinaryOp, lhs: Operand<'_>, rhs: Operand<'_>) -> ParseResult<bool>;
}

fn bytes(s: &OsStr) -> &[u8] {
    s.as_encoded_bytes()
}

fn token_is(s: &OsStr, token: &[u8]) -> bool {
    bytes(s) == token
}

fn is_bang(s: &OsStr) -> bool {
    token_is(s, b"!")
}

fn is_lparen(s: &OsStr) -> bool {
    token_is(s, b"(")
}

fn is_rparen(s: &OsStr) -> bool {
    token_is(s, b")")
}

fn is_and(s: &OsStr) -> bool {
    token_is(s, b"-a")
}

fn is_or(s: &OsStr) -> bool {
    token_is(s, b"-o")
}

fn is_length(s: &OsStr) -> bool {
    token_is(s, b"-l")
}

fn is_two_byte_switch(s: &OsStr) -> bool {
    let b = bytes(s);
    b.len() == 2 && b[0] == b'-' && b[1] != 0
}

fn binary_op(s: &OsStr) -> Option<BinaryOp> {
    Some(match bytes(s) {
        b"=" | b"==" => BinaryOp::StrEq,
        b"!=" => BinaryOp::StrNe,
        b"<" => BinaryOp::StrLt,
        b">" => BinaryOp::StrGt,
        b"-eq" => BinaryOp::IntEq,
        b"-ne" => BinaryOp::IntNe,
        b"-lt" => BinaryOp::IntLt,
        b"-le" => BinaryOp::IntLe,
        b"-gt" => BinaryOp::IntGt,
        b"-ge" => BinaryOp::IntGe,
        b"-ef" => BinaryOp::FileEf,
        b"-nt" => BinaryOp::FileNt,
        b"-ot" => BinaryOp::FileOt,
        _ => return None,
    })
}

fn unary_op(s: &OsStr) -> Option<UnaryOp> {
    Some(match bytes(s) {
        b"-b" => UnaryOp::BlockSpecial,
        b"-c" => UnaryOp::CharacterSpecial,
        b"-d" => UnaryOp::Directory,
        b"-e" => UnaryOp::Exists,
        b"-f" => UnaryOp::Regular,
        b"-g" => UnaryOp::GroupIdFlag,
        b"-G" => UnaryOp::GroupOwns,
        b"-h" | b"-L" => UnaryOp::SymLink,
        b"-k" => UnaryOp::Sticky,
        b"-N" => UnaryOp::ModifiedSinceRead,
        b"-O" => UnaryOp::UserOwns,
        b"-p" => UnaryOp::Fifo,
        b"-r" => UnaryOp::Readable,
        b"-s" => UnaryOp::NonEmpty,
        b"-S" => UnaryOp::Socket,
        b"-t" => UnaryOp::Tty,
        b"-u" => UnaryOp::UserIdFlag,
        b"-w" => UnaryOp::Writable,
        b"-x" => UnaryOp::Executable,
        b"-n" => UnaryOp::StrNonEmpty,
        b"-z" => UnaryOp::StrEmpty,
        _ => return None,
    })
}

/// Parser for `test` expressions.
///
/// `test` has special, historical meanings for expressions with one to four
/// arguments. Longer expressions use the following grammar, with the usual
/// precedence from strongest to weakest: `!`, `-a`, then `-o`.
///
/// ```text
/// EXPR       := OR_EXPR
/// OR_EXPR    := AND_EXPR ( "-o" AND_EXPR )*
/// AND_EXPR   := TERM ( "-a" TERM )*
/// TERM       := "!"* ATOM
/// ATOM       := "(" EXPR ")"
///            | VALUE OP VALUE
///            | "-l" VALUE INT_OP VALUE
///            | VALUE INT_OP "-l" VALUE
///            | UNARY_OP VALUE
///            | VALUE
/// OP         := STRING_OP | INT_OP | FILE_OP
/// STRING_OP  := "=" | "==" | "!=" | "<" | ">"
/// INT_OP     := "-eq" | "-ne" | "-lt" | "-le" | "-gt" | "-ge"
/// FILE_OP    := "-ef" | "-nt" | "-ot"
/// UNARY_OP   := "-n" | "-z" | FILE_TEST
/// FILE_TEST  := "-b" | "-c" | "-d" | "-e" | "-f" | "-g" | "-G" | "-h"
///            | "-k" | "-L" | "-N" | "-O" | "-p" | "-r" | "-s" | "-S"
///            | "-t" | "-u" | "-w" | "-x"
///```
///
/// The grammar is only a description of the long-expression path. Operators
/// are recognized according to their position, so tokens such as `!`, `-a`,
/// `-o`, and `(` can still be literal operands in the short forms. Parsing and
/// evaluation happen together, directly from the argument list; this preserves
/// `test`'s requirement to evaluate both sides of `-a` and `-o`.
struct Parser<'a, E> {
    args: &'a [OsString],
    pos: usize,
    eval: E,
}

impl<'a, E: Evaluator> Parser<'a, E> {
    fn arg(&self, index: usize) -> &'a OsStr {
        self.args[index].as_os_str()
    }

    #[cold]
    fn missing_after_last(&self) -> ParseError {
        let index = self.args.len().saturating_sub(1);
        let value = self
            .args
            .get(index)
            .map_or_else(|| "''".to_owned(), |s| s.quote().to_string());
        ParseError::at_token(ParseErrorKind::MissingArgument(value), index)
    }

    fn advance_required(&mut self) -> ParseResult<()> {
        self.pos += 1;
        if self.pos >= self.args.len() {
            Err(self.missing_after_last())
        } else {
            Ok(())
        }
    }

    fn eval_one(&mut self) -> bool {
        let result = !self.arg(self.pos).is_empty();
        self.pos += 1;
        result
    }

    fn eval_two(&mut self) -> ParseResult<bool> {
        if is_bang(self.arg(self.pos)) {
            self.pos += 1;
            return Ok(!self.eval_one());
        }

        // In this form every two-byte -X is treated as a unary operator first.
        // That is why an unknown switch is an error here, not a string literal.
        if is_two_byte_switch(self.arg(self.pos)) {
            return self.eval_unary();
        }

        Err(self.missing_after_last())
    }

    fn eval_three(&mut self) -> ParseResult<bool> {
        if let Some(op) = binary_op(self.arg(self.pos + 1)) {
            return self.eval_binary(false, op);
        }

        if is_bang(self.arg(self.pos)) {
            self.advance_required()?;
            return Ok(!self.eval_two()?);
        }

        if is_lparen(self.arg(self.pos)) && is_rparen(self.arg(self.pos + 2)) {
            self.pos += 1;
            let result = self.eval_one();
            self.pos += 1;
            return Ok(result);
        }

        if is_and(self.arg(self.pos + 1))
            || is_or(self.arg(self.pos + 1))
            || token_is(self.arg(self.pos + 1), b">")
            || token_is(self.arg(self.pos + 1), b"<")
        {
            return self.eval_expr();
        }

        let index = self.pos + 1;
        Err(ParseError::at_token(
            ParseErrorKind::BinaryOperatorExpected(self.arg(index).quote().to_string()),
            index,
        ))
    }

    /// Apply the historical one- to four-argument rules before using the
    /// precedence-based parser for longer expressions.
    fn eval_by_arity(&mut self, nargs: usize) -> ParseResult<bool> {
        match nargs {
            1 => Ok(self.eval_one()),
            2 => self.eval_two(),
            3 => self.eval_three(),
            4 => {
                if is_bang(self.arg(self.pos)) {
                    self.advance_required()?;
                    return Ok(!self.eval_three()?);
                }

                if is_lparen(self.arg(self.pos)) && is_rparen(self.arg(self.pos + 3)) {
                    self.pos += 1;
                    let result = self.eval_two()?;
                    self.pos += 1;
                    return Ok(result);
                }

                self.eval_expr()
            }
            _ => self.eval_expr(),
        }
    }

    /// Parse a long expression using `-a` precedence over `-o`.
    fn eval_expr(&mut self) -> ParseResult<bool> {
        if self.pos >= self.args.len() {
            return Err(self.missing_after_last());
        }
        self.eval_or()
    }

    fn eval_or(&mut self) -> ParseResult<bool> {
        let mut result = false;
        loop {
            // `test` evaluates both sides of -o, even when the result is known.
            result |= self.eval_and()?;
            if self.pos >= self.args.len() || !is_or(self.arg(self.pos)) {
                return Ok(result);
            }
            self.pos += 1;
        }
    }

    fn eval_and(&mut self) -> ParseResult<bool> {
        let mut result = true;
        loop {
            // `test` evaluates both sides of -a, even when the result is known.
            result &= self.eval_term()?;
            if self.pos >= self.args.len() || !is_and(self.arg(self.pos)) {
                return Ok(result);
            }
            self.pos += 1;
        }
    }

    /// Parse a term, consuming leading negations and then an atom.
    fn eval_term(&mut self) -> ParseResult<bool> {
        let mut negate = false;
        while self.pos < self.args.len() && is_bang(self.arg(self.pos)) {
            self.advance_required()?;
            negate = !negate;
        }

        if self.pos >= self.args.len() {
            return Err(self.missing_after_last());
        }

        let remaining = self.args.len() - self.pos;
        let value = if is_lparen(self.arg(self.pos)) {
            self.eval_parenthesized()?
        } else if remaining >= 4
            && is_length(self.arg(self.pos))
            && binary_op(self.arg(self.pos + 2)).is_some()
        {
            let op = binary_op(self.arg(self.pos + 2)).unwrap();
            self.eval_binary(true, op)?
        } else if remaining >= 3 {
            if let Some(op) = binary_op(self.arg(self.pos + 1)) {
                self.eval_binary(false, op)?
            } else if self.is_general_unary_start() {
                self.eval_unary()?
            } else {
                self.eval_literal()
            }
        } else if self.is_general_unary_start() {
            self.eval_unary()?
        } else {
            self.eval_literal()
        };

        Ok(negate ^ value)
    }

    fn is_general_unary_start(&self) -> bool {
        if !is_two_byte_switch(self.arg(self.pos)) {
            return false;
        }
        let b = bytes(self.arg(self.pos));
        b[1] != b'a' && b[1] != b'o'
    }

    fn eval_literal(&mut self) -> bool {
        let value = !self.arg(self.pos).is_empty();
        self.pos += 1;
        value
    }

    /// Parse a parenthesized expression while preserving short-form arity
    /// rules inside the parentheses.
    fn eval_parenthesized(&mut self) -> ParseResult<bool> {
        self.advance_required()?;

        // For short parenthesized expressions the arity rules still matter.
        // Once there are more than four arguments, the normal parser takes over.
        let mut nargs = 1usize;
        // A closing parenthesis can be the right-hand string operand in
        // `( ( != ) )`, so do not mistake it for the group's delimiter.
        let right_paren_is_operand = self.pos + 3 < self.args.len()
            && is_lparen(self.arg(self.pos))
            && binary_op(self.arg(self.pos + 1)).is_some()
            && is_rparen(self.arg(self.pos + 2))
            && is_rparen(self.arg(self.pos + 3));
        while self.pos + nargs < self.args.len()
            && (!is_rparen(self.arg(self.pos + nargs)) || (right_paren_is_operand && nargs == 2))
        {
            if nargs == 4 {
                nargs = self.args.len() - self.pos;
                break;
            }
            nargs += 1;
        }

        let result = self.eval_by_arity(nargs)?;
        if self.pos >= self.args.len() {
            return Err(ParseError::at_token(
                ParseErrorKind::Expected(OsStr::new(")").quote().to_string()),
                self.args.len().saturating_sub(1),
            ));
        }
        if !is_rparen(self.arg(self.pos)) {
            return Err(ParseError::at_token(
                ParseErrorKind::ExpectedFound(
                    OsStr::new(")").quote().to_string(),
                    self.arg(self.pos).quote().to_string(),
                ),
                self.pos,
            ));
        }
        self.pos += 1;
        Ok(result)
    }

    fn eval_unary(&mut self) -> ParseResult<bool> {
        let op_index = self.pos;
        let op_token = self.arg(op_index);
        let Some(op) = unary_op(op_token) else {
            return Err(ParseError::at_token(
                ParseErrorKind::UnaryOperatorExpected(op_token.quote().to_string()),
                op_index,
            ));
        };

        self.advance_required()?;
        let arg = self.arg(self.pos);
        self.pos += 1;
        self.eval.unary(op, arg)
    }

    fn eval_binary(&mut self, lhs_is_length: bool, op: BinaryOp) -> ParseResult<bool> {
        let start = self.pos;
        let op_index = start + if lhs_is_length { 2 } else { 1 };
        let rhs_index = op_index + 1;
        let rhs_is_length = op_index + 2 < self.args.len() && is_length(self.arg(rhs_index));

        // A right-hand -l consumes its string before the operator kind is
        // considered. Keep that oddity because it is visible in expressions.
        self.pos = op_index + if rhs_is_length { 3 } else { 2 };

        match op {
            BinaryOp::IntEq
            | BinaryOp::IntNe
            | BinaryOp::IntLt
            | BinaryOp::IntLe
            | BinaryOp::IntGt
            | BinaryOp::IntGe => {
                let lhs = if lhs_is_length {
                    Operand::Length(self.arg(start + 1))
                } else {
                    Operand::Value(self.arg(start))
                };
                let rhs = if rhs_is_length {
                    Operand::Length(self.arg(op_index + 2))
                } else {
                    Operand::Value(self.arg(rhs_index))
                };
                self.eval.binary(op, lhs, rhs)
            }
            BinaryOp::FileEf | BinaryOp::FileNt | BinaryOp::FileOt => {
                if lhs_is_length || rhs_is_length {
                    return Err(ParseError::at_token(
                        ParseErrorKind::DoesNotAcceptLength(op.as_str().to_owned()),
                        op_index,
                    ));
                }
                self.eval.binary(
                    op,
                    Operand::Value(self.arg(start)),
                    Operand::Value(self.arg(rhs_index)),
                )
            }
            BinaryOp::StrEq | BinaryOp::StrNe | BinaryOp::StrLt | BinaryOp::StrGt => {
                let lhs_index = if lhs_is_length { start + 1 } else { start };
                self.eval.binary(
                    op,
                    Operand::Value(self.arg(lhs_index)),
                    Operand::Value(self.arg(rhs_index)),
                )
            }
        }
    }

    fn finish(mut self) -> ParseResult<bool> {
        if self.args.is_empty() {
            return Ok(false);
        }

        let result = self.eval_by_arity(self.args.len())?;
        if self.pos != self.args.len() {
            return Err(ParseError::at_token(
                ParseErrorKind::ExtraArgument(self.arg(self.pos).quote().to_string()),
                self.pos,
            ));
        }
        Ok(result)
    }
}

pub(crate) fn evaluate<E: Evaluator>(args: &[OsString], eval: E) -> ParseResult<bool> {
    Parser { args, pos: 0, eval }.finish()
}
