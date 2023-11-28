// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ints paren prec multibytes

use num_bigint::BigInt;
use onig::{Regex, RegexOptions, Syntax};

use crate::{ExprError, ExprResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Relation(RelationOp),
    Numeric(NumericOp),
    String(StringOp),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationOp {
    Lt,
    Leq,
    Eq,
    Neq,
    Gt,
    Geq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringOp {
    Match,
    Index,
    And,
    Or,
}

impl BinOp {
    fn eval(&self, left: &AstNode, right: &AstNode) -> ExprResult<String> {
        match self {
            Self::Relation(op) => op.eval(left, right),
            Self::Numeric(op) => op.eval(left, right),
            Self::String(op) => op.eval(left, right),
        }
    }
}

impl RelationOp {
    fn eval(&self, a: &AstNode, b: &AstNode) -> ExprResult<String> {
        let a = a.eval()?;
        let b = b.eval()?;
        let b = if let (Ok(a), Ok(b)) = (a.parse::<BigInt>(), b.parse::<BigInt>()) {
            match self {
                Self::Lt => a < b,
                Self::Leq => a <= b,
                Self::Eq => a == b,
                Self::Neq => a != b,
                Self::Gt => a > b,
                Self::Geq => a >= b,
            }
        } else {
            // These comparisons should be using locale settings
            match self {
                Self::Lt => a < b,
                Self::Leq => a <= b,
                Self::Eq => a == b,
                Self::Neq => a != b,
                Self::Gt => a > b,
                Self::Geq => a >= b,
            }
        };
        if b {
            Ok("1".into())
        } else {
            Ok("0".into())
        }
    }
}

impl NumericOp {
    fn eval(&self, left: &AstNode, right: &AstNode) -> ExprResult<String> {
        let a: BigInt = left
            .eval()?
            .parse()
            .map_err(|_| ExprError::NonIntegerArgument)?;
        let b: BigInt = right
            .eval()?
            .parse()
            .map_err(|_| ExprError::NonIntegerArgument)?;
        Ok(match self {
            Self::Add => a + b,
            Self::Sub => a - b,
            Self::Mul => a * b,
            Self::Div => match a.checked_div(&b) {
                Some(x) => x,
                None => return Err(ExprError::DivisionByZero),
            },
            Self::Mod => {
                if a.checked_div(&b).is_none() {
                    return Err(ExprError::DivisionByZero);
                };
                a % b
            }
        }
        .to_string())
    }
}

impl StringOp {
    fn eval(&self, left: &AstNode, right: &AstNode) -> ExprResult<String> {
        match self {
            Self::Or => {
                let left = left.eval()?;
                if is_truthy(&left) {
                    return Ok(left);
                }
                let right = right.eval()?;
                if is_truthy(&right) {
                    return Ok(right);
                }
                Ok("0".into())
            }
            Self::And => {
                let left = left.eval()?;
                if !is_truthy(&left) {
                    return Ok("0".into());
                }
                let right = right.eval()?;
                if !is_truthy(&right) {
                    return Ok("0".into());
                }
                Ok(left)
            }
            Self::Match => {
                let left = left.eval()?;
                let right = right.eval()?;
                let re_string = format!("^{}", &right);
                let re = Regex::with_options(
                    &re_string,
                    RegexOptions::REGEX_OPTION_NONE,
                    Syntax::grep(),
                )
                .map_err(|_| ExprError::InvalidRegexExpression)?;
                Ok(if re.captures_len() > 0 {
                    re.captures(&left)
                        .map(|captures| captures.at(1).unwrap())
                        .unwrap_or("")
                        .to_string()
                } else {
                    re.find(&left)
                        .map_or("0".to_string(), |(start, end)| (end - start).to_string())
                })
            }
            Self::Index => {
                let left = left.eval()?;
                let right = right.eval()?;
                for (current_idx, ch_h) in left.chars().enumerate() {
                    for ch_n in right.chars() {
                        if ch_n == ch_h {
                            return Ok((current_idx + 1).to_string());
                        }
                    }
                }
                Ok("0".to_string())
            }
        }
    }
}

/// Precedence for infix binary operators
const PRECEDENCE: &[&[(&str, BinOp)]] = &[
    &[("|", BinOp::String(StringOp::Or))],
    &[("&", BinOp::String(StringOp::And))],
    &[
        ("<", BinOp::Relation(RelationOp::Lt)),
        ("<=", BinOp::Relation(RelationOp::Leq)),
        ("=", BinOp::Relation(RelationOp::Eq)),
        ("!=", BinOp::Relation(RelationOp::Neq)),
        (">=", BinOp::Relation(RelationOp::Geq)),
        (">", BinOp::Relation(RelationOp::Gt)),
    ],
    &[
        ("+", BinOp::Numeric(NumericOp::Add)),
        ("-", BinOp::Numeric(NumericOp::Sub)),
    ],
    &[
        ("*", BinOp::Numeric(NumericOp::Mul)),
        ("/", BinOp::Numeric(NumericOp::Div)),
        ("%", BinOp::Numeric(NumericOp::Mod)),
    ],
    &[(":", BinOp::String(StringOp::Match))],
];

#[derive(Debug, PartialEq, Eq)]
pub enum AstNode {
    Leaf {
        value: String,
    },
    BinOp {
        op_type: BinOp,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    Substr {
        string: Box<AstNode>,
        pos: Box<AstNode>,
        length: Box<AstNode>,
    },
    Length {
        string: Box<AstNode>,
    },
}

impl AstNode {
    pub fn parse(input: &[&str]) -> ExprResult<Self> {
        Parser::new(input).parse()
    }

    pub fn eval(&self) -> ExprResult<String> {
        match self {
            Self::Leaf { value } => Ok(value.into()),
            Self::BinOp {
                op_type,
                left,
                right,
            } => op_type.eval(left, right),
            Self::Substr {
                string,
                pos,
                length,
            } => {
                let string = string.eval()?;

                // The GNU docs say:
                //
                // > If either position or length is negative, zero, or
                // > non-numeric, returns the null string.
                //
                // So we coerce errors into 0 to make that the only case we
                // have to care about.
                let pos: usize = pos.eval()?.parse().unwrap_or(0);
                let length: usize = length.eval()?.parse().unwrap_or(0);

                let (Some(pos), Some(_)) = (pos.checked_sub(1), length.checked_sub(1)) else {
                    return Ok(String::new());
                };

                Ok(string.chars().skip(pos).take(length).collect())
            }
            Self::Length { string } => Ok(string.eval()?.chars().count().to_string()),
        }
    }
}

struct Parser<'a> {
    input: &'a [&'a str],
    index: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a [&'a str]) -> Self {
        Self { input, index: 0 }
    }

    fn next(&mut self) -> ExprResult<&'a str> {
        let next = self.input.get(self.index);
        if let Some(next) = next {
            self.index += 1;
            Ok(next)
        } else {
            // The indexing won't panic, because we know that the input size
            // is greater than zero.
            Err(ExprError::MissingArgument(
                self.input[self.index - 1].into(),
            ))
        }
    }

    fn accept<T>(&mut self, f: impl Fn(&str) -> Option<T>) -> Option<T> {
        let next = self.input.get(self.index)?;
        let tok = f(next);
        if let Some(tok) = tok {
            self.index += 1;
            Some(tok)
        } else {
            None
        }
    }

    fn parse(&mut self) -> ExprResult<AstNode> {
        if self.input.is_empty() {
            return Err(ExprError::MissingOperand);
        }
        let res = self.parse_expression()?;
        if let Some(arg) = self.input.get(self.index) {
            return Err(ExprError::UnexpectedArgument(arg.to_string()));
        }
        Ok(res)
    }

    fn parse_expression(&mut self) -> ExprResult<AstNode> {
        self.parse_precedence(0)
    }

    fn parse_op(&mut self, precedence: usize) -> Option<BinOp> {
        self.accept(|s| {
            for (op_string, op) in PRECEDENCE[precedence] {
                if s == *op_string {
                    return Some(*op);
                }
            }
            None
        })
    }

    fn parse_precedence(&mut self, precedence: usize) -> ExprResult<AstNode> {
        if precedence >= PRECEDENCE.len() {
            return self.parse_simple_expression();
        }

        let mut left = self.parse_precedence(precedence + 1)?;
        while let Some(op) = self.parse_op(precedence) {
            let right = self.parse_precedence(precedence + 1)?;
            left = AstNode::BinOp {
                op_type: op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_simple_expression(&mut self) -> ExprResult<AstNode> {
        let first = self.next()?;
        Ok(match first {
            "match" => {
                let left = self.parse_expression()?;
                let right = self.parse_expression()?;
                AstNode::BinOp {
                    op_type: BinOp::String(StringOp::Match),
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            "substr" => {
                let string = self.parse_expression()?;
                let pos = self.parse_expression()?;
                let length = self.parse_expression()?;
                AstNode::Substr {
                    string: Box::new(string),
                    pos: Box::new(pos),
                    length: Box::new(length),
                }
            }
            "index" => {
                let left = self.parse_expression()?;
                let right = self.parse_expression()?;
                AstNode::BinOp {
                    op_type: BinOp::String(StringOp::Index),
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            "length" => {
                let string = self.parse_expression()?;
                AstNode::Length {
                    string: Box::new(string),
                }
            }
            "+" => AstNode::Leaf {
                value: self.next()?.into(),
            },
            "(" => {
                let s = self.parse_expression()?;
                let close_paren = self.next()?;
                if close_paren != ")" {
                    // Since we have parsed at least a '(', there will be a token
                    // at `self.index - 1`. So this indexing won't panic.
                    return Err(ExprError::ExpectedClosingBraceAfter(
                        self.input[self.index - 1].into(),
                    ));
                }
                s
            }
            s => AstNode::Leaf { value: s.into() },
        })
    }
}

/// Determine whether `expr` should evaluate the string as "truthy"
///
/// Truthy strings are either empty or match the regex "-?0+".
pub fn is_truthy(s: &str) -> bool {
    // Edge case: `-` followed by nothing is truthy
    if s == "-" {
        return true;
    }

    let mut bytes = s.bytes();

    // Empty string is falsy
    let Some(first) = bytes.next() else {
        return false;
    };

    let is_zero = (first == b'-' || first == b'0') && bytes.all(|b| b == b'0');
    !is_zero
}

#[cfg(test)]
mod test {
    use super::{AstNode, BinOp, NumericOp, RelationOp, StringOp};

    impl From<&str> for AstNode {
        fn from(value: &str) -> Self {
            Self::Leaf {
                value: value.into(),
            }
        }
    }

    fn op(op_type: BinOp, left: impl Into<AstNode>, right: impl Into<AstNode>) -> AstNode {
        AstNode::BinOp {
            op_type,
            left: Box::new(left.into()),
            right: Box::new(right.into()),
        }
    }

    fn length(string: impl Into<AstNode>) -> AstNode {
        AstNode::Length {
            string: Box::new(string.into()),
        }
    }

    fn substr(
        string: impl Into<AstNode>,
        pos: impl Into<AstNode>,
        length: impl Into<AstNode>,
    ) -> AstNode {
        AstNode::Substr {
            string: Box::new(string.into()),
            pos: Box::new(pos.into()),
            length: Box::new(length.into()),
        }
    }

    #[test]
    fn infix_operators() {
        let cases = [
            ("|", BinOp::String(StringOp::Or)),
            ("&", BinOp::String(StringOp::And)),
            ("<", BinOp::Relation(RelationOp::Lt)),
            ("<=", BinOp::Relation(RelationOp::Leq)),
            ("=", BinOp::Relation(RelationOp::Eq)),
            ("!=", BinOp::Relation(RelationOp::Neq)),
            (">=", BinOp::Relation(RelationOp::Geq)),
            (">", BinOp::Relation(RelationOp::Gt)),
            ("+", BinOp::Numeric(NumericOp::Add)),
            ("-", BinOp::Numeric(NumericOp::Sub)),
            ("*", BinOp::Numeric(NumericOp::Mul)),
            ("/", BinOp::Numeric(NumericOp::Div)),
            ("%", BinOp::Numeric(NumericOp::Mod)),
            (":", BinOp::String(StringOp::Match)),
        ];
        for (string, value) in cases {
            assert_eq!(AstNode::parse(&["1", string, "2"]), Ok(op(value, "1", "2")));
        }
    }

    #[test]
    fn other_operators() {
        assert_eq!(
            AstNode::parse(&["match", "1", "2"]),
            Ok(op(BinOp::String(StringOp::Match), "1", "2")),
        );
        assert_eq!(
            AstNode::parse(&["index", "1", "2"]),
            Ok(op(BinOp::String(StringOp::Index), "1", "2")),
        );
        assert_eq!(AstNode::parse(&["length", "1"]), Ok(length("1")),);
        assert_eq!(
            AstNode::parse(&["substr", "1", "2", "3"]),
            Ok(substr("1", "2", "3")),
        );
    }

    #[test]
    fn precedence() {
        assert_eq!(
            AstNode::parse(&["1", "+", "2", "*", "3"]),
            Ok(op(
                BinOp::Numeric(NumericOp::Add),
                "1",
                op(BinOp::Numeric(NumericOp::Mul), "2", "3")
            ))
        );
        assert_eq!(
            AstNode::parse(&["(", "1", "+", "2", ")", "*", "3"]),
            Ok(op(
                BinOp::Numeric(NumericOp::Mul),
                op(BinOp::Numeric(NumericOp::Add), "1", "2"),
                "3"
            ))
        );
        assert_eq!(
            AstNode::parse(&["1", "*", "2", "+", "3"]),
            Ok(op(
                BinOp::Numeric(NumericOp::Add),
                op(BinOp::Numeric(NumericOp::Mul), "1", "2"),
                "3"
            )),
        );
    }
}
