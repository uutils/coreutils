// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ints paren prec multibytes

use std::{cell::Cell, collections::BTreeMap};

use num_bigint::{BigInt, ParseBigIntError};
use num_traits::ToPrimitive;
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
    fn eval(
        &self,
        left: ExprResult<NumOrStr>,
        right: ExprResult<NumOrStr>,
    ) -> ExprResult<NumOrStr> {
        match self {
            Self::Relation(op) => op.eval(left, right),
            Self::Numeric(op) => op.eval(left, right),
            Self::String(op) => op.eval(left, right),
        }
    }
}

impl RelationOp {
    fn eval(&self, a: ExprResult<NumOrStr>, b: ExprResult<NumOrStr>) -> ExprResult<NumOrStr> {
        let a = a?;
        let b = b?;
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
        if b { Ok(1.into()) } else { Ok(0.into()) }
    }
}

impl NumericOp {
    fn eval(
        &self,
        left: ExprResult<NumOrStr>,
        right: ExprResult<NumOrStr>,
    ) -> ExprResult<NumOrStr> {
        let a = left?.eval_as_bigint()?;
        let b = right?.eval_as_bigint()?;
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
    fn eval(
        &self,
        left: ExprResult<NumOrStr>,
        right: ExprResult<NumOrStr>,
    ) -> ExprResult<NumOrStr> {
        match self {
            Self::Or => {
                let left = left?;
                if is_truthy(&left) {
                    return Ok(left);
                }
                let right = right?;
                if is_truthy(&right) {
                    return Ok(right);
                }
                Ok(0.into())
            }
            Self::And => {
                let left = left?;
                if !is_truthy(&left) {
                    return Ok(0.into());
                }
                let right = right?;
                if !is_truthy(&right) {
                    return Ok(0.into());
                }
                Ok(left)
            }
            Self::Match => {
                let left = left?.eval_as_string();
                let right = right?.eval_as_string();
                check_posix_regex_errors(&right)?;

                // Transpile the input pattern from BRE syntax to `onig` crate's `Syntax::grep`
                let mut re_string = String::with_capacity(right.len() + 1);
                let mut pattern_chars = right.chars().peekable();
                let mut prev = '\0';
                let mut prev_is_escaped = false;
                let mut is_start_of_expression = true;

                // All patterns are anchored so they begin with a caret (^)
                if pattern_chars.peek() != Some(&'^') {
                    re_string.push('^');
                }

                while let Some(curr) = pattern_chars.next() {
                    let curr_is_escaped = prev == '\\' && !prev_is_escaped;
                    let is_first_character = prev == '\0';

                    match curr {
                        // Character class negation "[^a]"
                        // Explicitly escaped caret "\^"
                        '^' if !is_start_of_expression && !matches!(prev, '[' | '\\') => {
                            re_string.push_str(r"\^");
                        }
                        '$' if !curr_is_escaped && !is_end_of_expression(&pattern_chars) => {
                            re_string.push_str(r"\$");
                        }
                        '\\' if !curr_is_escaped && pattern_chars.peek().is_none() => {
                            return Err(ExprError::TrailingBackslash);
                        }
                        '{' if curr_is_escaped => {
                            // Handle '{' literally at the start of an expression
                            if is_start_of_expression {
                                if re_string.ends_with('\\') {
                                    let _ = re_string.pop();
                                }
                                re_string.push(curr);
                            } else {
                                // Check if the following section is a valid range quantifier
                                verify_range_quantifier(&pattern_chars)?;

                                re_string.push(curr);
                                // Set the lower bound of range quantifier to 0 if it is missing
                                if pattern_chars.peek() == Some(&',') {
                                    re_string.push('0');
                                }
                            }
                        }
                        _ => re_string.push(curr),
                    }

                    // Capturing group "\(abc\)"
                    // Alternative pattern "a\|b"
                    is_start_of_expression = curr == '\\' && is_first_character
                        || curr_is_escaped && matches!(curr, '(' | '|')
                        || curr == '\\' && prev_is_escaped && matches!(prev, '(' | '|');

                    prev_is_escaped = curr_is_escaped;
                    prev = curr;
                }

                let re = Regex::with_options(
                    &re_string,
                    RegexOptions::REGEX_OPTION_SINGLELINE,
                    Syntax::grep(),
                )
                .map_err(|error| match error.code() {
                    // "invalid repeat range {lower,upper}"
                    -123 => ExprError::InvalidBracketContent,
                    // "too big number for repeat range"
                    -201 => ExprError::TooBigRangeQuantifierIndex,
                    _ => ExprError::InvalidRegexExpression,
                })?;

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
                let left = left?.eval_as_string();
                let right = right?.eval_as_string();
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

/// Check if regex pattern character iterator is at the end of a regex expression or subexpression
fn is_end_of_expression<I>(pattern_chars: &I) -> bool
where
    I: Iterator<Item = char> + Clone,
{
    let mut pattern_chars_clone = pattern_chars.clone();
    match pattern_chars_clone.next() {
        Some('\\') => matches!(pattern_chars_clone.next(), Some(')' | '|')),
        None => true, // No characters left
        _ => false,
    }
}

/// Check if regex pattern character iterator is at the start of a valid range quantifier.
/// The iterator's start position is expected to be after the opening brace.
/// Range quantifier ends to closing brace.
///
/// # Examples of valid range quantifiers
///
/// - `r"\{3\}"`
/// - `r"\{3,\}"`
/// - `r"\{,6\}"`
/// - `r"\{3,6\}"`
/// - `r"\{,\}"`
fn verify_range_quantifier<I>(pattern_chars: &I) -> Result<(), ExprError>
where
    I: Iterator<Item = char> + Clone,
{
    let mut pattern_chars_clone = pattern_chars.clone().peekable();
    if pattern_chars_clone.peek().is_none() {
        return Err(ExprError::UnmatchedOpeningBrace);
    }

    // Parse the string between braces
    let mut quantifier = String::new();
    let mut prev = '\0';
    let mut curr_is_escaped = false;
    while let Some(curr) = pattern_chars_clone.next() {
        curr_is_escaped = prev == '\\' && !curr_is_escaped;
        if curr_is_escaped && curr == '}' {
            break;
        }
        if pattern_chars_clone.peek().is_none() {
            return Err(ExprError::UnmatchedOpeningBrace);
        }
        if prev != '\0' {
            quantifier.push(prev);
        }
        prev = curr;
    }

    // Check if parsed quantifier is valid
    let re = Regex::new(r"^([0-9]*,[0-9]*|[0-9]+)$").expect("valid regular expression");
    if let Some(captures) = re.captures(&quantifier) {
        let matched = captures.at(0).unwrap_or_default();
        match matched.split_once(',') {
            Some(("", "")) => Ok(()),
            Some((x, "") | ("", x)) if x.parse::<i16>().is_ok() => Ok(()),
            Some((_, "") | ("", _)) => Err(ExprError::TooBigRangeQuantifierIndex),
            Some((f, l)) => match (f.parse::<i16>(), l.parse::<i16>()) {
                (Ok(f), Ok(l)) if f > l => Err(ExprError::InvalidBracketContent),
                (Ok(_), Ok(_)) => Ok(()),
                _ => Err(ExprError::TooBigRangeQuantifierIndex),
            },
            None if matched.parse::<i16>().is_ok() => Ok(()),
            None => Err(ExprError::TooBigRangeQuantifierIndex),
        }
    } else {
        Err(ExprError::InvalidBracketContent)
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
    let mut prev = '\0';
    let mut curr_is_escaped = false;

    for curr in pattern.chars() {
        curr_is_escaped = prev == '\\' && !curr_is_escaped;
        match (curr_is_escaped, curr) {
            (true, '(') => escaped_parens += 1,
            (true, ')') => {
                escaped_parens = escaped_parens
                    .checked_sub(1)
                    .ok_or(ExprError::UnmatchedClosingParenthesis)?;
            }
            _ => {}
        }
        prev = curr;
    }

    match escaped_parens {
        0 => Ok(()),
        _ => Err(ExprError::UnmatchedOpeningParenthesis),
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

#[derive(Debug, Clone)]
pub struct AstNode {
    id: u32,
    inner: AstNodeInner,
}

// We derive Eq and PartialEq only for tests because we want to ignore the id field.
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum AstNodeInner {
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
    pub fn parse(input: &[impl AsRef<str>]) -> ExprResult<Self> {
        Parser::new(input).parse()
    }

    pub fn evaluated(self) -> ExprResult<Self> {
        Ok(Self {
            id: get_next_id(),
            inner: AstNodeInner::Evaluated {
                value: self.eval()?,
            },
        })
    }

    pub fn eval(&self) -> ExprResult<NumOrStr> {
        // This function implements a recursive tree-walking algorithm, but uses an explicit
        // stack approach instead of native recursion to avoid potential stack overflow
        // on deeply nested expressions.

        let mut stack = vec![self];
        let mut result_stack = BTreeMap::new();

        while let Some(node) = stack.pop() {
            match &node.inner {
                AstNodeInner::Evaluated { value, .. } => {
                    result_stack.insert(node.id, Ok(value.clone()));
                }
                AstNodeInner::Leaf { value, .. } => {
                    result_stack.insert(node.id, Ok(value.to_string().into()));
                }
                AstNodeInner::BinOp {
                    op_type,
                    left,
                    right,
                } => {
                    let (Some(right), Some(left)) = (
                        result_stack.remove(&right.id),
                        result_stack.remove(&left.id),
                    ) else {
                        stack.push(node);
                        stack.push(right);
                        stack.push(left);
                        continue;
                    };

                    let result = op_type.eval(left, right);
                    result_stack.insert(node.id, result);
                }
                AstNodeInner::Substr {
                    string,
                    pos,
                    length,
                } => {
                    let (Some(string), Some(pos), Some(length)) = (
                        result_stack.remove(&string.id),
                        result_stack.remove(&pos.id),
                        result_stack.remove(&length.id),
                    ) else {
                        stack.push(node);
                        stack.push(string);
                        stack.push(pos);
                        stack.push(length);
                        continue;
                    };

                    let string: String = string?.eval_as_string();

                    // The GNU docs say:
                    //
                    // > If either position or length is negative, zero, or
                    // > non-numeric, returns the null string.
                    //
                    // So we coerce errors into 0 to make that the only case we
                    // have to care about.
                    let pos = pos?
                        .eval_as_bigint()
                        .ok()
                        .and_then(|n| n.to_usize())
                        .unwrap_or(0);
                    let length = length?
                        .eval_as_bigint()
                        .ok()
                        .and_then(|n| n.to_usize())
                        .unwrap_or(0);

                    if let (Some(pos), Some(_)) = (pos.checked_sub(1), length.checked_sub(1)) {
                        let result = string.chars().skip(pos).take(length).collect::<String>();
                        result_stack.insert(node.id, Ok(result.into()));
                    } else {
                        result_stack.insert(node.id, Ok(String::new().into()));
                    }
                }
                AstNodeInner::Length { string } => {
                    // Push onto the stack

                    let Some(string) = result_stack.remove(&string.id) else {
                        stack.push(node);
                        stack.push(string);
                        continue;
                    };

                    let length = string?.eval_as_string().chars().count();
                    result_stack.insert(node.id, Ok(length.into()));
                }
            }
        }

        // The final result should be the only one left on the result stack
        result_stack.remove(&self.id).unwrap()
    }
}

thread_local! {
    static NODE_ID: Cell<u32> = const { Cell::new(1) };
}

// We create unique identifiers for each node in the AST.
// This is used to transform the recursive algorithm into an iterative one.
// It is used to store the result of each node's evaluation in a BtreeMap.
fn get_next_id() -> u32 {
    NODE_ID.with(|id| {
        let current = id.get();
        id.set(current + 1);
        current
    })
}

struct Parser<'a, S: AsRef<str>> {
    input: &'a [S],
    index: usize,
}

impl<'a, S: AsRef<str>> Parser<'a, S> {
    fn new(input: &'a [S]) -> Self {
        Self { input, index: 0 }
    }

    fn next(&mut self) -> ExprResult<&'a str> {
        let next = self.input.get(self.index);
        if let Some(next) = next {
            self.index += 1;
            Ok(next.as_ref())
        } else {
            // The indexing won't panic, because we know that the input size
            // is greater than zero.
            Err(ExprError::MissingArgument(
                self.input[self.index - 1].as_ref().into(),
            ))
        }
    }

    fn accept<T>(&mut self, f: impl Fn(&str) -> Option<T>) -> Option<T> {
        let next = self.input.get(self.index)?;
        let tok = f(next.as_ref());
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
            return Err(ExprError::UnexpectedArgument(arg.as_ref().into()));
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
            left = AstNode {
                id: get_next_id(),
                inner: AstNodeInner::BinOp {
                    op_type: op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn parse_simple_expression(&mut self) -> ExprResult<AstNode> {
        let first = self.next()?;
        let inner = match first {
            "match" => {
                let left = self.parse_expression()?;
                let right = self.parse_expression()?;
                AstNodeInner::BinOp {
                    op_type: BinOp::String(StringOp::Match),
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            "substr" => {
                let string = self.parse_expression()?;
                let pos = self.parse_expression()?;
                let length = self.parse_expression()?;
                AstNodeInner::Substr {
                    string: Box::new(string),
                    pos: Box::new(pos),
                    length: Box::new(length),
                }
            }
            "index" => {
                let left = self.parse_expression()?;
                let right = self.parse_expression()?;
                AstNodeInner::BinOp {
                    op_type: BinOp::String(StringOp::Index),
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            "length" => {
                let string = self.parse_expression()?;
                AstNodeInner::Length {
                    string: Box::new(string),
                }
            }
            "+" => AstNodeInner::Leaf {
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
                            self.input[self.index - 1].as_ref().into(),
                        ));
                    }
                    Err(ExprError::MissingArgument(_)) => {
                        return Err(ExprError::ExpectedClosingBraceAfter(
                            self.input[self.index - 1].as_ref().into(),
                        ));
                    }
                    Err(e) => return Err(e),
                }
                s.inner
            }
            s => AstNodeInner::Leaf { value: s.into() },
        };
        Ok(AstNode {
            id: get_next_id(),
            inner,
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
    use crate::syntax_tree::verify_range_quantifier;

    use super::{
        AstNode, AstNodeInner, BinOp, NumericOp, RelationOp, StringOp, check_posix_regex_errors,
        get_next_id,
    };

    impl PartialEq for AstNode {
        fn eq(&self, other: &Self) -> bool {
            self.inner == other.inner
        }
    }

    impl Eq for AstNode {}

    impl From<&str> for AstNode {
        fn from(value: &str) -> Self {
            Self {
                id: get_next_id(),
                inner: AstNodeInner::Leaf {
                    value: value.into(),
                },
            }
        }
    }

    fn op(op_type: BinOp, left: impl Into<AstNode>, right: impl Into<AstNode>) -> AstNode {
        AstNode {
            id: get_next_id(),
            inner: AstNodeInner::BinOp {
                op_type,
                left: Box::new(left.into()),
                right: Box::new(right.into()),
            },
        }
    }

    fn length(string: impl Into<AstNode>) -> AstNode {
        AstNode {
            id: get_next_id(),
            inner: AstNodeInner::Length {
                string: Box::new(string.into()),
            },
        }
    }

    fn substr(
        string: impl Into<AstNode>,
        pos: impl Into<AstNode>,
        length: impl Into<AstNode>,
    ) -> AstNode {
        AstNode {
            id: get_next_id(),
            inner: AstNodeInner::Substr {
                string: Box::new(string.into()),
                pos: Box::new(pos.into()),
                length: Box::new(length.into()),
            },
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
        assert_eq!(AstNode::parse(&["length", "1"]), Ok(length("1")));
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
    }

    #[test]
    fn check_regex_missing_opening() {
        assert_eq!(
            check_posix_regex_errors(r"abc\)"),
            Err(ExprError::UnmatchedClosingParenthesis)
        );
    }

    #[test]
    fn test_is_valid_range_quantifier() {
        assert!(verify_range_quantifier(&"3\\}".chars()).is_ok());
        assert!(verify_range_quantifier(&"3,\\}".chars()).is_ok());
        assert!(verify_range_quantifier(&",6\\}".chars()).is_ok());
        assert!(verify_range_quantifier(&"3,6\\}".chars()).is_ok());
        assert!(verify_range_quantifier(&",\\}".chars()).is_ok());
        assert!(verify_range_quantifier(&"32767\\}anything".chars()).is_ok());
        assert_eq!(
            verify_range_quantifier(&"\\{3,6\\}".chars()),
            Err(ExprError::InvalidBracketContent)
        );
        assert_eq!(
            verify_range_quantifier(&"\\}".chars()),
            Err(ExprError::InvalidBracketContent)
        );
        assert_eq!(
            verify_range_quantifier(&"".chars()),
            Err(ExprError::UnmatchedOpeningBrace)
        );
        assert_eq!(
            verify_range_quantifier(&"3".chars()),
            Err(ExprError::UnmatchedOpeningBrace)
        );
        assert_eq!(
            verify_range_quantifier(&"3,".chars()),
            Err(ExprError::UnmatchedOpeningBrace)
        );
        assert_eq!(
            verify_range_quantifier(&",6".chars()),
            Err(ExprError::UnmatchedOpeningBrace)
        );
        assert_eq!(
            verify_range_quantifier(&"3,6".chars()),
            Err(ExprError::UnmatchedOpeningBrace)
        );
        assert_eq!(
            verify_range_quantifier(&",".chars()),
            Err(ExprError::UnmatchedOpeningBrace)
        );
        assert_eq!(
            verify_range_quantifier(&"32768\\}".chars()),
            Err(ExprError::TooBigRangeQuantifierIndex)
        );
    }
}
