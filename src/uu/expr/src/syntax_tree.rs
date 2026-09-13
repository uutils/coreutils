// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ints paren prec multibytes aaaabc

use std::{cell::Cell, collections::BTreeMap};

use num_bigint::BigInt;
use num_traits::ToPrimitive;
use uucore::regex::{Regex, RegexBuilder, bre_to_ere};

use crate::{
    ExprError, ExprResult,
    locale_aware::{
        locale_aware_index, locale_aware_length, locale_aware_substr, locale_comparison,
    },
};

pub(crate) type MaybeNonUtf8String = Vec<u8>;
pub(crate) type MaybeNonUtf8Str = [u8];

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
    fn eval(self, left: ExprResult<NumOrStr>, right: ExprResult<NumOrStr>) -> ExprResult<NumOrStr> {
        match self {
            Self::Relation(op) => op.eval(left, right),
            Self::Numeric(op) => op.eval(left, right),
            Self::String(op) => op.eval(left, right),
        }
    }
}

impl RelationOp {
    fn eval(self, a: ExprResult<NumOrStr>, b: ExprResult<NumOrStr>) -> ExprResult<NumOrStr> {
        // Make sure that the given comparison validates the relational operator.
        let check_cmp = |cmp| {
            use RelationOp::{Eq, Geq, Gt, Leq, Lt, Neq};
            use std::cmp::Ordering::{Equal, Greater, Less};
            matches!(
                (self, cmp),
                (Lt | Leq | Neq, Less) | (Leq | Eq | Geq, Equal) | (Gt | Geq | Neq, Greater)
            )
        };

        let a = a?;
        let b = b?;
        let b = if let (Some(a), Some(b)) = (&a.to_bigint(), &b.to_bigint()) {
            check_cmp(a.cmp(b))
        } else {
            // These comparisons should be using locale settings

            let a = a.eval_as_string();
            let b = b.eval_as_string();

            check_cmp(locale_comparison(&a, &b))
        };
        if b { Ok(1.into()) } else { Ok(0.into()) }
    }
}

impl NumericOp {
    fn eval(self, left: ExprResult<NumOrStr>, right: ExprResult<NumOrStr>) -> ExprResult<NumOrStr> {
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
                }
                a % b
            }
        }))
    }
}

impl StringOp {
    fn eval(self, left: ExprResult<NumOrStr>, right: ExprResult<NumOrStr>) -> ExprResult<NumOrStr> {
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
                let left_bytes = left?.eval_as_string();
                let right_bytes = right?.eval_as_string();
                evaluate_match_expression(left_bytes, right_bytes)
            }
            Self::Index => {
                let left = left?.eval_as_string();
                let right = right?.eval_as_string();

                Ok(locale_aware_index(&left, &right).into())
            }
        }
    }
}

/// Build a regex from a pattern string with locale-aware encoding
fn build_regex(pattern_bytes: Vec<u8>) -> ExprResult<Regex> {
    use uucore::i18n::UEncoding;

    let encoding = uucore::i18n::get_locale_encoding();

    // For pattern processing, we need to handle it based on locale
    let pattern_str = match encoding {
        UEncoding::Utf8 => String::from_utf8(pattern_bytes.clone())
            .unwrap_or_else(|_| String::from_utf8_lossy(&pattern_bytes).into()),
        UEncoding::Ascii => pattern_bytes.iter().map(|&b| b as char).collect(),
    };

    let re_string = bre_to_ere(&pattern_str, true)?;

    let regex = RegexBuilder::new(&format!("(?s){re_string}"))
        .oniguruma_mode(true)
        .leftmost_longest(true)
        .seek(true)
        .build()
        .map_err(uucore::regex::RegexError::from)?;

    Ok(regex)
}

/// Find matches in the input using the compiled regex
fn find_match(regex: Regex, left_bytes: Vec<u8>) -> String {
    use uucore::i18n::UEncoding;

    let encoding = uucore::i18n::get_locale_encoding();
    let has_captures = regex.captures_len() > 1;

    // Match against the input using the appropriate encoding
    match encoding {
        UEncoding::Utf8 => {
            // In UTF-8 locale, check if input is valid UTF-8
            if let Ok(left_str) = std::str::from_utf8(&left_bytes) {
                // Valid UTF-8, match as UTF-8
                if let Ok(Some(caps)) = regex.captures(left_str) {
                    return if has_captures {
                        // Get first capture group
                        caps.get(1)
                            .map_or(String::new(), |m| m.as_str().to_string())
                    } else {
                        // Count characters in the match
                        caps.get(0).unwrap().as_str().chars().count().to_string()
                    };
                }
            } else {
                // Invalid UTF-8 in UTF-8 locale: match on Latin-1 byte mapping
                let left_str: String = left_bytes.iter().map(|&b| b as char).collect();
                if let Ok(Some(caps)) = regex.captures(&left_str) {
                    if has_captures {
                        if let Some(m) = caps.get(1) {
                            let bytes: Vec<u8> = m.as_str().chars().map(|c| c as u8).collect();
                            // Return empty string for invalid UTF-8 capture in UTF-8 locale
                            if let Ok(s) = String::from_utf8(bytes) {
                                return s;
                            }
                        }
                        return String::new();
                    }
                    // No capture groups - return 0 for invalid UTF-8 in UTF-8 locale
                    return "0".to_string();
                }
            }
        }
        UEncoding::Ascii => {
            // In ASCII/C locale, work with Latin-1 byte mapping
            let left_str: String = left_bytes.iter().map(|&b| b as char).collect();
            if let Ok(Some(caps)) = regex.captures(&left_str) {
                return if has_captures {
                    caps.get(1).map_or_else(String::new, |m| {
                        let bytes: Vec<u8> = m.as_str().chars().map(|c| c as u8).collect();
                        String::from_utf8_lossy(&bytes).into_owned()
                    })
                } else {
                    caps.get(0).unwrap().as_str().chars().count().to_string()
                };
            }
        }
    }

    // No match
    if has_captures {
        String::new()
    } else {
        "0".to_string()
    }
}

/// Evaluate a match expression with locale-aware regex matching
fn evaluate_match_expression(left_bytes: Vec<u8>, right_bytes: Vec<u8>) -> ExprResult<NumOrStr> {
    use uucore::i18n::UEncoding;

    let regex = build_regex(right_bytes)?;

    // Special case for ASCII locale with capture groups that need to return raw bytes
    let encoding = uucore::i18n::get_locale_encoding();

    if matches!(encoding, UEncoding::Ascii) && regex.captures_len() > 1 {
        // Try to find the actual capture bytes for ASCII locale
        let left_str: String = left_bytes.iter().map(|&b| b as char).collect();
        if let Ok(Some(caps)) = regex.captures(&left_str)
            && let Some(m) = caps.get(1)
        {
            let bytes: Vec<u8> = m.as_str().chars().map(|c| c as u8).collect();
            return Ok(MaybeNonUtf8String::from(bytes).into());
        }
    }

    Ok(find_match(regex, left_bytes).into())
}

/// Precedence for infix binary operators
const PRECEDENCE: &[&[(&MaybeNonUtf8Str, BinOp)]] = &[
    &[(b"|", BinOp::String(StringOp::Or))],
    &[(b"&", BinOp::String(StringOp::And))],
    &[
        (b"<", BinOp::Relation(RelationOp::Lt)),
        (b"<=", BinOp::Relation(RelationOp::Leq)),
        (b"=", BinOp::Relation(RelationOp::Eq)),
        (b"!=", BinOp::Relation(RelationOp::Neq)),
        (b">=", BinOp::Relation(RelationOp::Geq)),
        (b">", BinOp::Relation(RelationOp::Gt)),
    ],
    &[
        (b"+", BinOp::Numeric(NumericOp::Add)),
        (b"-", BinOp::Numeric(NumericOp::Sub)),
    ],
    &[
        (b"*", BinOp::Numeric(NumericOp::Mul)),
        (b"/", BinOp::Numeric(NumericOp::Div)),
        (b"%", BinOp::Numeric(NumericOp::Mod)),
    ],
    &[(b":", BinOp::String(StringOp::Match))],
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumOrStr {
    Num(BigInt),
    Str(MaybeNonUtf8String),
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
        Self::Str(str.into())
    }
}

impl From<MaybeNonUtf8String> for NumOrStr {
    fn from(str: MaybeNonUtf8String) -> Self {
        Self::Str(str)
    }
}

impl NumOrStr {
    pub fn to_bigint(&self) -> Option<BigInt> {
        match self {
            Self::Num(num) => Some(num.clone()),
            Self::Str(str) => std::str::from_utf8(str).ok()?.parse::<BigInt>().ok(),
        }
    }

    pub fn eval_as_bigint(self) -> ExprResult<BigInt> {
        match self {
            Self::Num(num) => Ok(num),
            Self::Str(str) => std::str::from_utf8(&str)
                .ok()
                .and_then(|s| s.parse::<BigInt>().ok())
                .ok_or_else(|| ExprError::NonIntegerArgument(str.clone())),
        }
    }

    pub fn eval_as_string(self) -> MaybeNonUtf8String {
        match self {
            Self::Num(num) => num.to_string().into(),
            Self::Str(str) => str,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AstNode {
    id: u32,
    inner: AstNodeInner,
}

// Eq and PartialEq are implemented only for tests, ignoring the id and
// position fields.
#[derive(Debug, Clone)]
pub enum AstNodeInner {
    Leaf {
        value: MaybeNonUtf8String,
        /// Index of the argument the value came from, for diagnostics.
        position: usize,
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
    fn new(inner: AstNodeInner) -> Self {
        Self {
            id: get_next_id(),
            inner,
        }
    }

    /// Parse `input`, reporting on failure how many arguments the parser had
    /// consumed. Together with the error kind that is enough to say which
    /// argument is at fault.
    pub fn parse_located(
        input: &[impl AsRef<MaybeNonUtf8Str>],
    ) -> Result<Self, (ExprError, usize)> {
        let mut parser = Parser::new(input);
        parser.parse().map_err(|e| (e, parser.index))
    }

    /// [`AstNode::eval_located`] without the location, for tests that only
    /// care about the value.
    #[cfg(test)]
    pub fn eval(&self) -> ExprResult<NumOrStr> {
        self.eval_located().map_err(|(error, _)| error)
    }

    /// Evaluate, reporting on failure the index of the argument the error is
    /// about, when there is one single argument to blame.
    pub fn eval_located(&self) -> Result<NumOrStr, (ExprError, Option<usize>)> {
        // This function implements a recursive tree-walking algorithm, but uses an explicit
        // stack approach instead of native recursion to avoid potential stack overflow
        // on deeply nested expressions.

        let mut stack = vec![self];
        let mut result_stack: BTreeMap<u32, Result<NumOrStr, (ExprError, Option<usize>)>> =
            BTreeMap::new();

        while let Some(node) = stack.pop() {
            match &node.inner {
                AstNodeInner::Leaf { value, .. } => {
                    result_stack.insert(node.id, Ok(value.to_owned().into()));
                }
                AstNodeInner::BinOp {
                    op_type,
                    left,
                    right,
                } => {
                    let (Some(right_result), Some(left_result)) = (
                        result_stack.remove(&right.id),
                        result_stack.remove(&left.id),
                    ) else {
                        stack.push(node);
                        stack.push(right);
                        stack.push(left);
                        continue;
                    };

                    // The operator takes plain results — some, like `|`,
                    // swallow their children's errors — so the positions are
                    // held back and re-attached if an error comes out.
                    let (left_result, left_position) = split(left_result);
                    let (right_result, right_position) = split(right_result);
                    let result = op_type.eval(left_result, right_result).map_err(|error| {
                        let position = match &error {
                            // Born here from a leaf operand, unless a child
                            // already located it deeper in the expression.
                            ExprError::NonIntegerArgument(value) => left_position
                                .or(right_position)
                                .or_else(|| leaf_position(left, value))
                                .or_else(|| leaf_position(right, value)),
                            _ => left_position.or(right_position),
                        };
                        (error, position)
                    });
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

                    let string: MaybeNonUtf8String = string?.eval_as_string();

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
                        let result = locale_aware_substr(string, pos, length);
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

                    let length = locale_aware_length(&string?.eval_as_string());
                    result_stack.insert(node.id, Ok(length.into()));
                }
            }
        }

        // The final result should be the only one left on the result stack
        result_stack.remove(&self.id).unwrap()
    }
}

/// Take the position out of a located result, leaving the plain result the
/// operator evaluators work on.
fn split(
    result: Result<NumOrStr, (ExprError, Option<usize>)>,
) -> (ExprResult<NumOrStr>, Option<usize>) {
    match result {
        Ok(value) => (Ok(value), None),
        Err((error, position)) => (Err(error), position),
    }
}

/// The position of `node`, when it is a leaf holding exactly `value`.
fn leaf_position(node: &AstNode, value: &MaybeNonUtf8Str) -> Option<usize> {
    match &node.inner {
        AstNodeInner::Leaf {
            value: leaf,
            position,
        } if leaf == value => Some(*position),
        _ => None,
    }
}

impl Drop for AstNode {
    // This is a tree-walking algorithm, so like `eval` it uses an explicit
    // stack instead of native recursion to avoid a stack overflow when
    // dropping a deeply nested AST.
    fn drop(&mut self) {
        fn detach_children(inner: &mut AstNodeInner, stack: &mut Vec<AstNode>) {
            let empty = AstNodeInner::Leaf {
                value: Vec::new(),
                position: 0,
            };
            match std::mem::replace(inner, empty) {
                AstNodeInner::Leaf { .. } => {}
                AstNodeInner::BinOp { left, right, .. } => {
                    stack.push(*left);
                    stack.push(*right);
                }
                AstNodeInner::Substr {
                    string,
                    pos,
                    length,
                } => {
                    stack.push(*string);
                    stack.push(*pos);
                    stack.push(*length);
                }
                AstNodeInner::Length { string } => stack.push(*string),
            }
        }

        let mut stack = Vec::new();
        detach_children(&mut self.inner, &mut stack);
        // The detached nodes are leaves by now, so dropping them at the end of
        // each iteration doesn't recurse.
        while let Some(mut node) = stack.pop() {
            detach_children(&mut node.inner, &mut stack);
        }
    }
}

thread_local! {
    static NODE_ID: Cell<u32> = const { Cell::new(1) };
}

/// We create unique identifiers for each node in the AST.
/// This is used to transform the recursive algorithm into an iterative one.
/// It is used to store the result of each node's evaluation in a `BtreeMap`.
fn get_next_id() -> u32 {
    NODE_ID.with(|id| {
        let current = id.get();
        id.set(current + 1);
        current
    })
}

/// A prefix keyword that takes a fixed number of simple-expression arguments
#[derive(Debug, Clone, Copy)]
enum Keyword {
    Match,
    Substr,
    Index,
    Length,
}

impl Keyword {
    fn arity(self) -> usize {
        match self {
            Self::Length => 1,
            Self::Match | Self::Index => 2,
            Self::Substr => 3,
        }
    }

    fn build(self, args: Vec<AstNode>) -> AstNodeInner {
        let mut args = args.into_iter();
        let mut next = || Box::new(args.next().expect("arity checked by caller"));
        match self {
            Self::Match => AstNodeInner::BinOp {
                op_type: BinOp::String(StringOp::Match),
                left: next(),
                right: next(),
            },
            Self::Substr => AstNodeInner::Substr {
                string: next(),
                pos: next(),
                length: next(),
            },
            Self::Index => AstNodeInner::BinOp {
                op_type: BinOp::String(StringOp::Index),
                left: next(),
                right: next(),
            },
            Self::Length => AstNodeInner::Length { string: next() },
        }
    }
}

/// What the parser has to parse next
enum ParseState {
    /// An expression containing operators of at least `min_prec` precedence
    Expression { min_prec: usize },
    /// A simple expression: a leaf token, or a keyword/parenthesized expression
    Simple,
    /// Nothing; a sub-expression has been fully parsed
    Value(AstNode),
}

/// Work to resume once the pending sub-expression is parsed
enum ParseFrame {
    /// Use the value as the left operand of the operator loop at `min_prec`
    ContinueExpression { min_prec: usize },
    /// Use the value as the right operand of `op`, then continue at `min_prec`
    CombineBinOp {
        min_prec: usize,
        op: BinOp,
        left: AstNode,
    },
    /// Use the value as the next argument of `keyword`
    KeywordArg {
        keyword: Keyword,
        args: Vec<AstNode>,
    },
    /// The value is a parenthesized expression; a closing parenthesis must follow
    CloseParen,
}

struct Parser<'a, S: AsRef<MaybeNonUtf8Str>> {
    input: &'a [S],
    index: usize,
}

impl<'a, S: AsRef<MaybeNonUtf8Str>> Parser<'a, S> {
    fn new(input: &'a [S]) -> Self {
        Self { input, index: 0 }
    }

    fn next(&mut self) -> ExprResult<&'a MaybeNonUtf8Str> {
        let next = self.input.get(self.index);
        if let Some(next) = next {
            self.index += 1;
            Ok(next.as_ref())
        } else {
            // The indexing won't panic, because we know that the input size
            // is greater than zero.
            Err(ExprError::MissingArgument(
                String::from_utf8_lossy(self.input[self.index - 1].as_ref()).into_owned(),
            ))
        }
    }

    fn accept<T>(&mut self, f: impl Fn(&MaybeNonUtf8Str) -> Option<T>) -> Option<T> {
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
            return Err(ExprError::UnexpectedArgument(
                String::from_utf8_lossy(arg.as_ref()).into_owned(),
            ));
        }
        Ok(res)
    }

    /// Accept the next token if it is an operator of at least `min_prec`
    /// precedence, returning the operator and its precedence
    fn parse_op(&mut self, min_prec: usize) -> Option<(usize, BinOp)> {
        self.accept(|s| {
            for (prec, ops) in PRECEDENCE.iter().enumerate().skip(min_prec) {
                for (op_string, op) in *ops {
                    if s == *op_string {
                        return Some((prec, *op));
                    }
                }
            }
            None
        })
    }

    // This is a recursive-descent algorithm (precedence climbing), but like
    // `eval` it uses an explicit stack instead of native recursion to avoid a
    // stack overflow on deeply nested expressions (e.g. thousands of nested
    // parentheses or `length` keywords).
    fn parse_expression(&mut self) -> ExprResult<AstNode> {
        let mut stack = Vec::new();
        let mut state = ParseState::Expression { min_prec: 0 };
        loop {
            state = match state {
                ParseState::Expression { min_prec } => {
                    stack.push(ParseFrame::ContinueExpression { min_prec });
                    ParseState::Simple
                }
                ParseState::Simple => match self.next()? {
                    b"match" => {
                        stack.push(ParseFrame::KeywordArg {
                            keyword: Keyword::Match,
                            args: Vec::new(),
                        });
                        ParseState::Simple
                    }
                    b"substr" => {
                        stack.push(ParseFrame::KeywordArg {
                            keyword: Keyword::Substr,
                            args: Vec::new(),
                        });
                        ParseState::Simple
                    }
                    b"index" => {
                        stack.push(ParseFrame::KeywordArg {
                            keyword: Keyword::Index,
                            args: Vec::new(),
                        });
                        ParseState::Simple
                    }
                    b"length" => {
                        stack.push(ParseFrame::KeywordArg {
                            keyword: Keyword::Length,
                            args: Vec::new(),
                        });
                        ParseState::Simple
                    }
                    b"+" => ParseState::Value(AstNode::new(AstNodeInner::Leaf {
                        value: self.next()?.into(),
                        position: self.index - 1,
                    })),
                    b"(" => {
                        stack.push(ParseFrame::CloseParen);
                        ParseState::Expression { min_prec: 0 }
                    }
                    s => ParseState::Value(AstNode::new(AstNodeInner::Leaf {
                        value: s.into(),
                        position: self.index - 1,
                    })),
                },
                ParseState::Value(value) => match stack.pop() {
                    None => return Ok(value),
                    Some(ParseFrame::ContinueExpression { min_prec }) => {
                        if let Some((prec, op)) = self.parse_op(min_prec) {
                            stack.push(ParseFrame::CombineBinOp {
                                min_prec,
                                op,
                                left: value,
                            });
                            ParseState::Expression { min_prec: prec + 1 }
                        } else {
                            ParseState::Value(value)
                        }
                    }
                    Some(ParseFrame::CombineBinOp { min_prec, op, left }) => {
                        stack.push(ParseFrame::ContinueExpression { min_prec });
                        ParseState::Value(AstNode::new(AstNodeInner::BinOp {
                            op_type: op,
                            left: Box::new(left),
                            right: Box::new(value),
                        }))
                    }
                    Some(ParseFrame::KeywordArg { keyword, mut args }) => {
                        args.push(value);
                        if args.len() < keyword.arity() {
                            stack.push(ParseFrame::KeywordArg { keyword, args });
                            ParseState::Simple
                        } else {
                            ParseState::Value(AstNode::new(keyword.build(args)))
                        }
                    }
                    Some(ParseFrame::CloseParen) => match self.next() {
                        Ok(b")") => ParseState::Value(value),
                        // Since we have parsed at least a '(', there will be a token
                        // at `self.index - 1`. So this indexing won't panic.
                        Ok(_) => {
                            return Err(ExprError::ExpectedClosingBraceInsteadOf(
                                String::from_utf8_lossy(self.input[self.index - 1].as_ref()).into(),
                            ));
                        }
                        Err(ExprError::MissingArgument(_)) => {
                            return Err(ExprError::ExpectedClosingBraceAfter(
                                String::from_utf8_lossy(self.input[self.index - 1].as_ref()).into(),
                            ));
                        }
                        Err(e) => return Err(e),
                    },
                },
            };
        }
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
            if str == b"-" {
                return true;
            }

            let mut bytes = str.iter().copied();

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
    use crate::{ExprError, ExprResult};
    use uucore::regex::RegexError;

    use super::{
        AstNode, AstNodeInner, BinOp, MaybeNonUtf8Str, NumericOp, RelationOp, StringOp, get_next_id,
    };

    /// Parse an expression, discarding how far the parser got.
    fn parse<S: AsRef<MaybeNonUtf8Str>>(input: &[S]) -> ExprResult<AstNode> {
        AstNode::parse_located(input).map_err(|(e, _)| e)
    }

    impl PartialEq for AstNode {
        fn eq(&self, other: &Self) -> bool {
            self.inner == other.inner
        }
    }

    impl Eq for AstNode {}

    // Hand-built expectations cannot know real argument positions, so
    // equality ignores them, like it ignores ids.
    impl PartialEq for AstNodeInner {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (Self::Leaf { value: a, .. }, Self::Leaf { value: b, .. }) => a == b,
                (
                    Self::BinOp {
                        op_type: a_op,
                        left: a_left,
                        right: a_right,
                    },
                    Self::BinOp {
                        op_type: b_op,
                        left: b_left,
                        right: b_right,
                    },
                ) => a_op == b_op && a_left == b_left && a_right == b_right,
                (
                    Self::Substr {
                        string: a_string,
                        pos: a_pos,
                        length: a_length,
                    },
                    Self::Substr {
                        string: b_string,
                        pos: b_pos,
                        length: b_length,
                    },
                ) => a_string == b_string && a_pos == b_pos && a_length == b_length,
                (Self::Length { string: a }, Self::Length { string: b }) => a == b,
                _ => false,
            }
        }
    }

    impl Eq for AstNodeInner {}

    impl From<&str> for AstNode {
        fn from(value: &str) -> Self {
            Self {
                id: get_next_id(),
                inner: AstNodeInner::Leaf {
                    value: value.into(),
                    position: 0,
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
            assert_eq!(parse(&["1", string, "2"]), Ok(op(value, "1", "2")));
        }
    }

    #[test]
    fn other_operators() {
        assert_eq!(
            parse(&["match", "1", "2"]),
            Ok(op(BinOp::String(StringOp::Match), "1", "2")),
        );
        assert_eq!(
            parse(&["index", "1", "2"]),
            Ok(op(BinOp::String(StringOp::Index), "1", "2")),
        );
        assert_eq!(parse(&["length", "1"]), Ok(length("1")));
        assert_eq!(parse(&["substr", "1", "2", "3"]), Ok(substr("1", "2", "3")),);
    }

    #[test]
    fn precedence() {
        assert_eq!(
            parse(&["1", "+", "2", "*", "3"]),
            Ok(op(
                BinOp::Numeric(NumericOp::Add),
                "1",
                op(BinOp::Numeric(NumericOp::Mul), "2", "3")
            ))
        );
        assert_eq!(
            parse(&["(", "1", "+", "2", ")", "*", "3"]),
            Ok(op(
                BinOp::Numeric(NumericOp::Mul),
                op(BinOp::Numeric(NumericOp::Add), "1", "2"),
                "3"
            ))
        );
        assert_eq!(
            parse(&["1", "*", "2", "+", "3"]),
            Ok(op(
                BinOp::Numeric(NumericOp::Add),
                op(BinOp::Numeric(NumericOp::Mul), "1", "2"),
                "3"
            )),
        );
    }

    #[test]
    fn deeply_nested_parse_eval_drop() {
        // Deeply nested expressions should parse, evaluate and drop without
        // overflowing the stack.
        let depth = 100_000;
        let mut input: Vec<&str> = vec!["("; depth];
        input.push("1");
        input.extend(std::iter::repeat_n(")", depth));
        let result = parse(&input).unwrap().eval().unwrap();
        assert_eq!(result.eval_as_string(), b"1");

        let mut input: Vec<&str> = vec!["length"; depth];
        input.push("1");
        let result = parse(&input).unwrap().eval().unwrap();
        assert_eq!(result.eval_as_string(), b"1");
    }

    #[test]
    fn missing_closing_parenthesis() {
        assert_eq!(
            parse(&["(", "42"]),
            Err(ExprError::ExpectedClosingBraceAfter("42".to_string()))
        );
        assert_eq!(
            parse(&["(", "42", "a"]),
            Err(ExprError::ExpectedClosingBraceInsteadOf("a".to_string()))
        );
    }

    #[test]
    fn empty_substitution() {
        // causes a panic in 0.0.25
        let result = parse(&["a", ":", r"\(b\)*"]).unwrap().eval().unwrap();
        assert_eq!(result.eval_as_string(), b"");
    }

    #[test]
    fn starting_stars_become_escaped() {
        let result = parse(&["cats", ":", r"*cats"]).unwrap().eval().unwrap();
        assert_eq!(result.eval_as_string(), b"0");

        let result = parse(&["*cats", ":", r"*cats"]).unwrap().eval().unwrap();
        assert_eq!(result.eval_as_string(), b"5");
    }

    #[test]
    fn only_match_in_beginning() {
        let result = parse(&["budget", ":", r"get"]).unwrap().eval().unwrap();
        assert_eq!(result.eval_as_string(), b"0");
    }

    #[test]
    fn test_evaluate_match_expression_basic() {
        use super::evaluate_match_expression;

        // Basic literal match
        let result = evaluate_match_expression(b"hello".to_vec(), b"hello".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"5");

        // No match
        let result = evaluate_match_expression(b"hello".to_vec(), b"world".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"0");

        // Partial match from beginning
        let result = evaluate_match_expression(b"hello world".to_vec(), b"hello".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"5");
    }

    #[test]
    fn test_evaluate_match_expression_regex_patterns() {
        use super::evaluate_match_expression;

        // Dot matches any character
        let result = evaluate_match_expression(b"abc".to_vec(), b"a.c".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"3");

        // Star quantifier
        let result = evaluate_match_expression(b"aaaabc".to_vec(), b"a*bc".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"6");

        // Plus quantifier (escaped in BRE)
        let result = evaluate_match_expression(b"aaaabc".to_vec(), b"a\\+bc".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"6");

        // Question mark quantifier (escaped in BRE)
        let result = evaluate_match_expression(b"abc".to_vec(), b"ab\\?c".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"3");
    }

    #[test]
    fn test_evaluate_match_expression_capture_groups() {
        use super::evaluate_match_expression;

        // Simple capture group
        let result =
            evaluate_match_expression(b"hello123".to_vec(), b"hello\\([0-9]*\\)".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"123");

        // Empty capture group
        let result =
            evaluate_match_expression(b"hello".to_vec(), b"hello\\([0-9]*\\)".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"");

        // No capture group, just match length
        let result =
            evaluate_match_expression(b"hello123".to_vec(), b"hello[0-9]*".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"8");
    }

    #[test]
    fn test_evaluate_match_expression_character_classes() {
        use super::evaluate_match_expression;

        // Simple character class
        let result = evaluate_match_expression(b"abc123".to_vec(), b"[a-z]*".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"3");

        // Negated character class
        let result = evaluate_match_expression(b"123abc".to_vec(), b"[^a-z]*".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"3");

        // Digit character class
        let result = evaluate_match_expression(b"123abc".to_vec(), b"[0-9]*".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"3");
    }

    #[test]
    fn test_evaluate_match_expression_anchoring() {
        use super::evaluate_match_expression;

        // Patterns are automatically anchored at start
        let result = evaluate_match_expression(b"world hello".to_vec(), b"hello".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"0");

        // Explicit start anchor (redundant but should work)
        let result =
            evaluate_match_expression(b"hello world".to_vec(), b"^hello".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"5");

        // End anchor
        let result =
            evaluate_match_expression(b"hello world".to_vec(), b"world$".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"0"); // Should fail because not at start

        let result = evaluate_match_expression(b"world".to_vec(), b"world$".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"5");
    }

    #[test]
    fn test_evaluate_match_expression_special_characters() {
        use super::evaluate_match_expression;

        // Escaped special characters
        let result = evaluate_match_expression(b"a.b".to_vec(), b"a\\.b".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"3");

        // Escaped asterisk
        let result = evaluate_match_expression(b"a*b".to_vec(), b"a\\*b".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"3");

        // Caret not at beginning should be escaped
        let result = evaluate_match_expression(b"a^b".to_vec(), b"a^b".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"3");

        // Dollar not at end should be escaped
        let result = evaluate_match_expression(b"a$b".to_vec(), b"a$b".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"3");
    }

    #[test]
    fn test_evaluate_match_expression_range_quantifiers() {
        use super::evaluate_match_expression;

        // Fixed count quantifier
        let result = evaluate_match_expression(b"aaa".to_vec(), b"a\\{3\\}".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"3");

        // Range quantifier
        let result = evaluate_match_expression(b"aa".to_vec(), b"a\\{1,3\\}".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"2");

        // Minimum quantifier
        let result = evaluate_match_expression(b"aaaa".to_vec(), b"a\\{2,\\}".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"4");

        // Maximum quantifier
        let result = evaluate_match_expression(b"aa".to_vec(), b"a\\{,3\\}".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"2");
    }

    #[test]
    fn test_evaluate_match_expression_empty_and_edge_cases() {
        use super::evaluate_match_expression;

        // Empty input string
        let result = evaluate_match_expression(b"".to_vec(), b".*".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"0");

        // Empty pattern (should match empty string)
        let result = evaluate_match_expression(b"".to_vec(), b"".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"0");

        // Pattern matching empty string
        let result = evaluate_match_expression(b"hello".to_vec(), b".*".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"5");
    }

    #[test]
    fn test_evaluate_match_expression_error_cases() {
        use super::evaluate_match_expression;

        // Unmatched opening parenthesis
        let result = evaluate_match_expression(b"hello".to_vec(), b"\\(hello".to_vec());
        assert!(matches!(
            result,
            Err(ExprError::Regex(RegexError::UnmatchedOpeningParenthesis))
        ));

        // Unmatched closing parenthesis
        let result = evaluate_match_expression(b"hello".to_vec(), b"hello\\)".to_vec());
        assert!(matches!(
            result,
            Err(ExprError::Regex(RegexError::UnmatchedClosingParenthesis))
        ));

        // Trailing backslash
        let result = evaluate_match_expression(b"hello".to_vec(), b"hello\\".to_vec());
        assert!(matches!(
            result,
            Err(ExprError::Regex(RegexError::TrailingBackslash))
        ));

        // Invalid bracket content
        let result = evaluate_match_expression(b"hello".to_vec(), b"a\\{invalid\\}".to_vec());
        assert!(matches!(
            result,
            Err(ExprError::Regex(RegexError::InvalidBracketContent))
        ));
    }

    #[test]
    fn test_evaluate_match_expression_multibyte_character_class() {
        use super::evaluate_match_expression;
        use uucore::i18n::{UEncoding, get_locale_encoding};

        let result = evaluate_match_expression(
            vec![0xce, 0xb1, b'b', b'c', 0xce, 0xb4, b'e', b'f'],
            vec![b'[', 0xce, 0xb1, b']'],
        )
        .unwrap();
        assert_eq!(result.eval_as_string(), b"1");

        let result = evaluate_match_expression(
            vec![0xce, 0xb1, b'b', b'c', 0xce, 0xb4, b'e', b'f'],
            vec![b'\\', b'(', b'[', 0xce, 0xb1, b']', b'\\', b')'],
        )
        .unwrap();
        match get_locale_encoding() {
            UEncoding::Utf8 => assert_eq!(result.eval_as_string(), &[0xce, 0xb1]),
            UEncoding::Ascii => assert_eq!(result.eval_as_string(), &[0xce]),
        }
    }

    #[test]
    fn test_adjacent_quantifiers() {
        use super::evaluate_match_expression;

        let result = evaluate_match_expression(b"aaa".to_vec(), br"\(a\)\{2\}*".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"a");

        let result = evaluate_match_expression(b"aaa".to_vec(), br"a**".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"3");
    }

    #[test]
    fn test_leftmost_longest_match_semantics() {
        use super::evaluate_match_expression;

        // This test verifies leftmost-longest (POSIX) match semantics.
        // Pattern `\(a\|ab\)` against `ab` should capture the longest alternative (`ab`),
        // not the first (`a`).
        let result = evaluate_match_expression(b"ab".to_vec(), br"\(a\|ab\)".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"ab");

        // `aaaaa\|a*` against `aaaaaa` should match all 6 characters
        let result = evaluate_match_expression(b"aaaaaa".to_vec(), br"aaaaa\|a*".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"6");
    }

    #[test]
    fn test_gnu_bre_extensions_and_escaped_caret() {
        use super::evaluate_match_expression;

        // Word character \w
        let result = evaluate_match_expression(b"a1b".to_vec(), br"\w".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"1");

        // Word boundary \b
        let result = evaluate_match_expression(b"abc".to_vec(), br"\ba".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"1");

        // Beginning of word \<
        let result = evaluate_match_expression(b"abc".to_vec(), br"\<a".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"1");

        // End of word \>
        let result = evaluate_match_expression(b"b".to_vec(), br"b\>".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"1");

        // Beginning of buffer \`
        let result = evaluate_match_expression(b"start".to_vec(), br"\`start".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"5");

        // End of buffer \'
        let result = evaluate_match_expression(b"end".to_vec(), br"end\'".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"3");

        // Escaped caret with quantifier \^*
        let result = evaluate_match_expression(b"^".to_vec(), br"\^*".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"1");
    }

    #[test]
    fn test_invalid_utf8_with_high_bytes_before_capture() {
        use super::evaluate_match_expression;
        use uucore::i18n::{UEncoding, get_locale_encoding};

        // Input contains non-UTF-8 byte >= 0x80 (0xFF) before a valid ASCII capture
        let result =
            evaluate_match_expression(vec![0xff, b'a', b'b', b'c'], br".\(abc\)".to_vec()).unwrap();
        assert_eq!(result.eval_as_string(), b"abc");

        // In UTF-8 locale, capture group containing invalid UTF-8 returns empty string;
        // in ASCII/C locale (e.g. WASI default), raw bytes are captured.
        let result =
            evaluate_match_expression(vec![0xff, b'a', b'b', b'c'], br"\(.*\)".to_vec()).unwrap();
        match get_locale_encoding() {
            UEncoding::Utf8 => assert_eq!(result.eval_as_string(), b""),
            UEncoding::Ascii => assert_eq!(result.eval_as_string(), &[0xff, b'a', b'b', b'c']),
        }
    }
}
