// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ints paren prec multibytes

use num_bigint::{BigInt, ParseBigIntError};
use num_traits::{ToPrimitive, Zero};
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
    fn eval(&self, left: &AstNode, right: &AstNode) -> ExprResult<NumOrStr> {
        match self {
            Self::Relation(op) => op.eval(left, right),
            Self::Numeric(op) => op.eval(left, right),
            Self::String(op) => op.eval(left, right),
        }
    }
}

impl RelationOp {
    fn eval(&self, a: &AstNode, b: &AstNode) -> ExprResult<NumOrStr> {
        let a = a.eval()?;
        let b = b.eval()?;
        let b = if let (Ok(a), Ok(b)) = (&a.to_bigint(), &b.to_bigint()) {
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
            let a = a.eval_as_string();
            let b = b.eval_as_string();
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
            Ok(1.into())
        } else {
            Ok(0.into())
        }
    }
}

impl NumericOp {
    fn eval(&self, left: &AstNode, right: &AstNode) -> ExprResult<NumOrStr> {
        let a = left.eval()?.eval_as_bigint()?;
        let b = right.eval()?.eval_as_bigint()?;
        Ok(NumOrStr::Num(match self {
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
        }))
    }
}

impl StringOp {
    fn eval(&self, left: &AstNode, right: &AstNode) -> ExprResult<NumOrStr> {
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
                Ok(0.into())
            }
            Self::And => {
                let left = left.eval()?;
                if !is_truthy(&left) {
                    return Ok(0.into());
                }
                let right = right.eval()?;
                if !is_truthy(&right) {
                    return Ok(0.into());
                }
                Ok(left)
            }
            Self::Match => {
                let left = left.eval()?.eval_as_string();
                let right = right.eval()?.eval_as_string();
                check_posix_regex_errors(&right)?;
                let prefix = if right.starts_with('*') { r"^\" } else { "^" };
                let re_string = format!("{prefix}{right}");
                let re = Regex::with_options(
                    &re_string,
                    RegexOptions::REGEX_OPTION_NONE,
                    Syntax::grep(),
                )
                .map_err(|_| ExprError::InvalidRegexExpression)?;
                Ok(if re.captures_len() > 0 {
                    re.captures(&left)
                        .and_then(|captures| captures.at(1))
                        .unwrap_or("")
                        .to_string()
                } else {
                    re.find(&left)
                        .map_or("0".to_string(), |(start, end)| (end - start).to_string())
                }
                .into())
            }
            Self::Index => {
                let left = left.eval()?.eval_as_string();
                let right = right.eval()?.eval_as_string();
                for (current_idx, ch_h) in left.chars().enumerate() {
                    for ch_n in right.to_string().chars() {
                        if ch_n == ch_h {
                            return Ok((current_idx + 1).into());
                        }
                    }
                }
                Ok(0.into())
            }
        }
    }
}

/// Check for errors in a supplied regular expression
///
/// GNU coreutils shows messages for invalid regular expressions
/// differently from the oniguruma library used by the regex crate.
/// This method attempts to do these checks manually in one pass
/// through the regular expression.
///
/// This method is not comprehensively checking all cases in which
/// a regular expression could be invalid; any cases not caught will
/// result in a [ExprError::InvalidRegexExpression] when passing the
/// regular expression through the Oniguruma bindings. This method is
/// intended to just identify a few situations for which GNU coreutils
/// has specific error messages.
fn check_posix_regex_errors(pattern: &str) -> ExprResult<()> {
    let mut escaped_parens: u64 = 0;
    let mut escaped_braces: u64 = 0;
    let mut escaped = false;

    let mut repeating_pattern_text = String::new();
    let mut invalid_content_error = false;

    for c in pattern.chars() {
        match (escaped, c) {
            (true, ')') => {
                escaped_parens = escaped_parens
                    .checked_sub(1)
                    .ok_or(ExprError::UnmatchedClosingParenthesis)?;
            }
            (true, '(') => {
                escaped_parens += 1;
            }
            (true, '}') => {
                escaped_braces = escaped_braces
                    .checked_sub(1)
                    .ok_or(ExprError::UnmatchedClosingBrace)?;
                let mut repetition =
                    repeating_pattern_text[..repeating_pattern_text.len() - 1].splitn(2, ',');
                match (
                    repetition
                        .next()
                        .expect("splitn always returns at least one string"),
                    repetition.next(),
                ) {
                    ("", None) => {
                        // Empty repeating pattern
                        invalid_content_error = true;
                    }
                    (x, None) | (x, Some("")) => {
                        if x.parse::<i16>().is_err() {
                            invalid_content_error = true;
                        }
                    }
                    ("", Some(x)) => {
                        if x.parse::<i16>().is_err() {
                            invalid_content_error = true;
                        }
                    }
                    (f, Some(l)) => {
                        if let (Ok(f), Ok(l)) = (f.parse::<i16>(), l.parse::<i16>()) {
                            invalid_content_error = invalid_content_error || f > l;
                        } else {
                            invalid_content_error = true;
                        }
                    }
                }
                repeating_pattern_text.clear();
            }
            (true, '{') => {
                escaped_braces += 1;
            }
            _ => {
                if escaped_braces > 0 && repeating_pattern_text.len() <= 13 {
                    repeating_pattern_text.push(c);
                }
                if escaped_braces > 0 && !(c.is_ascii_digit() || c == '\\' || c == ',') {
                    invalid_content_error = true;
                }
            }
        }
        escaped = !escaped && c == '\\';
    }
    match (
        escaped_parens.is_zero(),
        escaped_braces.is_zero(),
        invalid_content_error,
    ) {
        (true, true, false) => Ok(()),
        (_, false, _) => Err(ExprError::UnmatchedOpeningBrace),
        (false, _, _) => Err(ExprError::UnmatchedOpeningParenthesis),
        (true, true, true) => Err(ExprError::InvalidContent(r"\{\}".to_string())),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumOrStr {
    Num(BigInt),
    Str(String),
}

impl From<usize> for NumOrStr {
    fn from(num: usize) -> Self {
        Self::Num(BigInt::from(num))
    }
}

impl From<BigInt> for NumOrStr {
    fn from(num: BigInt) -> Self {
        Self::Num(num)
    }
}

impl From<String> for NumOrStr {
    fn from(str: String) -> Self {
        Self::Str(str)
    }
}

impl NumOrStr {
    pub fn to_bigint(&self) -> Result<BigInt, ParseBigIntError> {
        match self {
            Self::Num(num) => Ok(num.clone()),
            Self::Str(str) => str.parse::<BigInt>(),
        }
    }

    pub fn eval_as_bigint(self) -> ExprResult<BigInt> {
        match self {
            Self::Num(num) => Ok(num),
            Self::Str(str) => str
                .parse::<BigInt>()
                .map_err(|_| ExprError::NonIntegerArgument),
        }
    }

    pub fn eval_as_string(self) -> String {
        match self {
            Self::Num(num) => num.to_string(),
            Self::Str(str) => str,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AstNode {
    Evaluated {
        value: NumOrStr,
    },
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

    pub fn evaluated(self) -> ExprResult<Self> {
        Ok(Self::Evaluated {
            value: self.eval()?,
        })
    }

    pub fn eval(&self) -> ExprResult<NumOrStr> {
        match self {
            Self::Evaluated { value } => Ok(value.clone()),
            Self::Leaf { value } => Ok(value.to_string().into()),
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
                let string: String = string.eval()?.eval_as_string();

                // The GNU docs say:
                //
                // > If either position or length is negative, zero, or
                // > non-numeric, returns the null string.
                //
                // So we coerce errors into 0 to make that the only case we
                // have to care about.
                let pos = pos
                    .eval()?
                    .eval_as_bigint()
                    .ok()
                    .and_then(|n| n.to_usize())
                    .unwrap_or(0);
                let length = length
                    .eval()?
                    .eval_as_bigint()
                    .ok()
                    .and_then(|n| n.to_usize())
                    .unwrap_or(0);

                let (Some(pos), Some(_)) = (pos.checked_sub(1), length.checked_sub(1)) else {
                    return Ok(String::new().into());
                };

                Ok(string
                    .chars()
                    .skip(pos)
                    .take(length)
                    .collect::<String>()
                    .into())
            }
            Self::Length { string } => Ok(string.eval()?.eval_as_string().chars().count().into()),
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
                // Evaluate the node just after parsing to we detect arithmetic
                // errors before checking for the closing parenthesis.
                let s = self.parse_expression()?.evaluated()?;

                match self.next() {
                    Ok(")") => {}
                    // Since we have parsed at least a '(', there will be a token
                    // at `self.index - 1`. So this indexing won't panic.
                    Ok(_) => {
                        return Err(ExprError::ExpectedClosingBraceInsteadOf(
                            self.input[self.index - 1].into(),
                        ));
                    }
                    Err(ExprError::MissingArgument(_)) => {
                        return Err(ExprError::ExpectedClosingBraceAfter(
                            self.input[self.index - 1].into(),
                        ));
                    }
                    Err(e) => return Err(e),
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
pub fn is_truthy(s: &NumOrStr) -> bool {
    match s {
        NumOrStr::Num(num) => num != &BigInt::from(0),
        NumOrStr::Str(str) => {
            // Edge case: `-` followed by nothing is truthy
            if str == "-" {
                return true;
            }

            let mut bytes = str.bytes();

            // Empty string is falsy
            let Some(first) = bytes.next() else {
                return false;
            };

            let is_zero = (first == b'-' || first == b'0') && bytes.all(|b| b == b'0');
            !is_zero
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ExprError;
    use crate::ExprError::InvalidContent;

    use super::{check_posix_regex_errors, AstNode, BinOp, NumericOp, RelationOp, StringOp};

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
                op(BinOp::Numeric(NumericOp::Add), "1", "2")
                    .evaluated()
                    .unwrap(),
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

    #[test]
    fn missing_closing_parenthesis() {
        assert_eq!(
            AstNode::parse(&["(", "42"]),
            Err(ExprError::ExpectedClosingBraceAfter("42".to_string()))
        );
        assert_eq!(
            AstNode::parse(&["(", "42", "a"]),
            Err(ExprError::ExpectedClosingBraceInsteadOf("a".to_string()))
        );
    }

    #[test]
    fn empty_substitution() {
        // causes a panic in 0.0.25
        let result = AstNode::parse(&["a", ":", r"\(b\)*"])
            .unwrap()
            .eval()
            .unwrap();
        assert_eq!(result.eval_as_string(), "");
    }

    #[test]
    fn starting_stars_become_escaped() {
        let result = AstNode::parse(&["cats", ":", r"*cats"])
            .unwrap()
            .eval()
            .unwrap();
        assert_eq!(result.eval_as_string(), "0");

        let result = AstNode::parse(&["*cats", ":", r"*cats"])
            .unwrap()
            .eval()
            .unwrap();
        assert_eq!(result.eval_as_string(), "5");
    }

    #[test]
    fn only_match_in_beginning() {
        let result = AstNode::parse(&["budget", ":", r"get"])
            .unwrap()
            .eval()
            .unwrap();
        assert_eq!(result.eval_as_string(), "0");
    }

    #[test]
    fn check_regex_valid() {
        assert!(check_posix_regex_errors(r"(a+b) \(a* b\)").is_ok());
    }

    #[test]
    fn check_regex_simple_repeating_pattern() {
        assert!(check_posix_regex_errors(r"\(a+b\)\{4\}").is_ok());
    }

    #[test]
    fn check_regex_missing_closing() {
        assert_eq!(
            check_posix_regex_errors(r"\(abc"),
            Err(ExprError::UnmatchedOpeningParenthesis)
        );

        assert_eq!(
            check_posix_regex_errors(r"\{1,2"),
            Err(ExprError::UnmatchedOpeningBrace)
        );
    }

    #[test]
    fn check_regex_missing_opening() {
        assert_eq!(
            check_posix_regex_errors(r"abc\)"),
            Err(ExprError::UnmatchedClosingParenthesis)
        );

        assert_eq!(
            check_posix_regex_errors(r"abc\}"),
            Err(ExprError::UnmatchedClosingBrace)
        );
    }

    #[test]
    fn check_regex_empty_repeating_pattern() {
        assert_eq!(
            check_posix_regex_errors("ab\\{\\}"),
            Err(InvalidContent(r"\{\}".to_string()))
        )
    }

    #[test]
    fn check_regex_intervals_two_numbers() {
        assert_eq!(
            // out of order
            check_posix_regex_errors("ab\\{1,0\\}"),
            Err(InvalidContent(r"\{\}".to_string()))
        );
        assert_eq!(
            check_posix_regex_errors("ab\\{1,a\\}"),
            Err(InvalidContent(r"\{\}".to_string()))
        );
        assert_eq!(
            check_posix_regex_errors("ab\\{a,3\\}"),
            Err(InvalidContent(r"\{\}".to_string()))
        );
        assert_eq!(
            check_posix_regex_errors("ab\\{a,b\\}"),
            Err(InvalidContent(r"\{\}".to_string()))
        );
        assert_eq!(
            check_posix_regex_errors("ab\\{a,\\}"),
            Err(InvalidContent(r"\{\}".to_string()))
        );
        assert_eq!(
            check_posix_regex_errors("ab\\{,b\\}"),
            Err(InvalidContent(r"\{\}".to_string()))
        );
    }
}
