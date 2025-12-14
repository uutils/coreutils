// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ints paren prec multibytes aaaabc

use std::{cell::Cell, collections::BTreeMap};

use num_bigint::BigInt;
use num_traits::ToPrimitive;
use onig::{Regex, RegexOptions, Syntax};

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
                }
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
/// result in a [`ExprError::InvalidRegexExpression`] when passing the
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

/// Build a regex from a pattern string with locale-aware encoding
fn build_regex(pattern_bytes: Vec<u8>) -> ExprResult<(Regex, String)> {
    use onig::EncodedBytes;
    use uucore::i18n::{UEncoding, get_locale_encoding};

    let encoding = get_locale_encoding();

    // For pattern processing, we need to handle it based on locale
    let pattern_str = String::from_utf8(pattern_bytes.clone())
        .unwrap_or_else(|_| String::from_utf8_lossy(&pattern_bytes).into());
    check_posix_regex_errors(&pattern_str)?;

    // Transpile the input pattern from BRE syntax to `onig` crate's `Syntax::grep`
    let mut re_string = String::with_capacity(pattern_str.len() + 1);
    let mut pattern_chars = pattern_str.chars().peekable();
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

    // Create regex with proper encoding
    let re = match encoding {
        UEncoding::Utf8 => {
            // For UTF-8 locale, use UTF-8 encoding
            Regex::with_options_and_encoding(
                &re_string,
                RegexOptions::REGEX_OPTION_SINGLELINE,
                Syntax::grep(),
            )
        }
        UEncoding::Ascii => {
            // For non-UTF-8 locale, use ASCII encoding
            Regex::with_options_and_encoding(
                EncodedBytes::ascii(re_string.as_bytes()),
                RegexOptions::REGEX_OPTION_SINGLELINE,
                Syntax::grep(),
            )
        }
    }
    .map_err(|error| match error.code() {
        // "invalid repeat range {lower,upper}"
        -123 => ExprError::InvalidBracketContent,
        // "too big number for repeat range"
        -201 => ExprError::TooBigRangeQuantifierIndex,
        _ => ExprError::InvalidRegexExpression,
    })?;

    Ok((re, re_string))
}

/// Find matches in the input using the compiled regex
fn find_match(regex: Regex, re_string: String, left_bytes: Vec<u8>) -> ExprResult<String> {
    use onig::EncodedBytes;
    use uucore::i18n::{UEncoding, get_locale_encoding};

    let encoding = get_locale_encoding();

    // Match against the input using the appropriate encoding
    let mut region = onig::Region::new();
    let result = match encoding {
        UEncoding::Utf8 => {
            // In UTF-8 locale, check if input is valid UTF-8
            if let Ok(left_str) = std::str::from_utf8(&left_bytes) {
                // Valid UTF-8, match as UTF-8
                let pos = regex.search_with_encoding(
                    left_str,
                    0,
                    left_str.len(),
                    onig::SearchOptions::SEARCH_OPTION_NONE,
                    Some(&mut region),
                );

                if pos.is_some() {
                    if regex.captures_len() > 0 {
                        // Get first capture group
                        region
                            .pos(1)
                            .map(|(start, end)| left_str[start..end].to_string())
                            .unwrap_or_default()
                    } else {
                        // Count characters in the match
                        let (start, end) = region.pos(0).unwrap();
                        left_str[start..end].chars().count().to_string()
                    }
                } else {
                    // No match
                    if regex.captures_len() > 0 {
                        String::new()
                    } else {
                        "0".to_string()
                    }
                }
            } else {
                // Invalid UTF-8 in UTF-8 locale
                // Try to match as bytes using ASCII encoding
                let left_encoded = EncodedBytes::ascii(&left_bytes);
                // Need to create ASCII version of regex too
                let re_ascii = Regex::with_options_and_encoding(
                    EncodedBytes::ascii(re_string.as_bytes()),
                    RegexOptions::REGEX_OPTION_SINGLELINE,
                    Syntax::grep(),
                )
                .ok();

                if let Some(re_ascii) = re_ascii {
                    let pos = re_ascii.search_with_encoding(
                        left_encoded,
                        0,
                        left_bytes.len(),
                        onig::SearchOptions::SEARCH_OPTION_NONE,
                        Some(&mut region),
                    );

                    if pos.is_some() {
                        if re_ascii.captures_len() > 0 {
                            // Get first capture group
                            region
                                .pos(1)
                                .map(|(start, end)| {
                                    // Return empty string for invalid UTF-8 capture in UTF-8 locale
                                    if std::str::from_utf8(&left_bytes[start..end]).is_err() {
                                        String::new()
                                    } else {
                                        String::from_utf8_lossy(&left_bytes[start..end])
                                            .into_owned()
                                    }
                                })
                                .unwrap_or_default()
                        } else {
                            // No capture groups - return 0 for invalid UTF-8 in UTF-8 locale
                            "0".to_string()
                        }
                    } else {
                        // No match
                        if re_ascii.captures_len() > 0 {
                            String::new()
                        } else {
                            "0".to_string()
                        }
                    }
                } else {
                    // Couldn't create ASCII regex - no match
                    if regex.captures_len() > 0 {
                        String::new()
                    } else {
                        "0".to_string()
                    }
                }
            }
        }
        UEncoding::Ascii => {
            // In ASCII/C locale, work with bytes directly
            let left_encoded = EncodedBytes::ascii(&left_bytes);
            let pos = regex.search_with_encoding(
                left_encoded,
                0,
                left_bytes.len(),
                onig::SearchOptions::SEARCH_OPTION_NONE,
                Some(&mut region),
            );

            if pos.is_some() {
                if regex.captures_len() > 0 {
                    // Get first capture group - return raw bytes for C locale
                    if let Some((start, end)) = region.pos(1) {
                        let capture_bytes = &left_bytes[start..end];
                        // Return raw bytes as String for consistency with other cases
                        return Ok(String::from_utf8_lossy(capture_bytes).into_owned());
                    }
                    String::new()
                } else {
                    // Return byte count of match
                    let (start, end) = region.pos(0).unwrap();
                    (end - start).to_string()
                }
            } else {
                // No match
                if regex.captures_len() > 0 {
                    String::new()
                } else {
                    "0".to_string()
                }
            }
        }
    };

    Ok(result)
}

/// Evaluate a match expression with locale-aware regex matching
fn evaluate_match_expression(left_bytes: Vec<u8>, right_bytes: Vec<u8>) -> ExprResult<NumOrStr> {
    let (regex, re_string) = build_regex(right_bytes)?;

    // Special case for ASCII locale with capture groups that need to return raw bytes
    use uucore::i18n::{UEncoding, get_locale_encoding};
    let encoding = get_locale_encoding();

    if matches!(encoding, UEncoding::Ascii) && regex.captures_len() > 0 {
        // Try to find the actual capture bytes for ASCII locale
        let mut region = onig::Region::new();
        let left_encoded = onig::EncodedBytes::ascii(&left_bytes);
        let pos = regex.search_with_encoding(
            left_encoded,
            0,
            left_bytes.len(),
            onig::SearchOptions::SEARCH_OPTION_NONE,
            Some(&mut region),
        );

        if pos.is_some() {
            if let Some((start, end)) = region.pos(1) {
                let capture_bytes = &left_bytes[start..end];
                return Ok(MaybeNonUtf8String::from(capture_bytes.to_vec()).into());
            }
        }
    }

    let result = find_match(regex, re_string, left_bytes)?;
    Ok(result.into())
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
            Self::Str(str) => String::from_utf8(str)
                .map_err(|_| ExprError::NonIntegerArgument)?
                .parse::<BigInt>()
                .map_err(|_| ExprError::NonIntegerArgument),
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

// We derive Eq and PartialEq only for tests because we want to ignore the id field.
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum AstNodeInner {
    Evaluated {
        value: NumOrStr,
    },
    Leaf {
        value: MaybeNonUtf8String,
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
    pub fn parse(input: &[impl AsRef<MaybeNonUtf8Str>]) -> ExprResult<Self> {
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
                    result_stack.insert(node.id, Ok(value.to_owned().into()));
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
            b"match" => {
                let left = self.parse_simple_expression()?;
                let right = self.parse_simple_expression()?;
                AstNodeInner::BinOp {
                    op_type: BinOp::String(StringOp::Match),
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            b"substr" => {
                let string = self.parse_simple_expression()?;
                let pos = self.parse_simple_expression()?;
                let length = self.parse_simple_expression()?;
                AstNodeInner::Substr {
                    string: Box::new(string),
                    pos: Box::new(pos),
                    length: Box::new(length),
                }
            }
            b"index" => {
                let left = self.parse_simple_expression()?;
                let right = self.parse_simple_expression()?;
                AstNodeInner::BinOp {
                    op_type: BinOp::String(StringOp::Index),
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            b"length" => {
                let string = self.parse_simple_expression()?;
                AstNodeInner::Length {
                    string: Box::new(string),
                }
            }
            b"+" => AstNodeInner::Leaf {
                value: self.next()?.into(),
            },
            b"(" => {
                // Evaluate the node just after parsing to we detect arithmetic
                // errors before checking for the closing parenthesis.
                let s = self.parse_expression()?.evaluated()?;

                match self.next() {
                    Ok(b")") => {}
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
        assert_eq!(result.eval_as_string(), b"");
    }

    #[test]
    fn starting_stars_become_escaped() {
        let result = AstNode::parse(&["cats", ":", r"*cats"])
            .unwrap()
            .eval()
            .unwrap();
        assert_eq!(result.eval_as_string(), b"0");

        let result = AstNode::parse(&["*cats", ":", r"*cats"])
            .unwrap()
            .eval()
            .unwrap();
        assert_eq!(result.eval_as_string(), b"5");
    }

    #[test]
    fn only_match_in_beginning() {
        let result = AstNode::parse(&["budget", ":", r"get"])
            .unwrap()
            .eval()
            .unwrap();
        assert_eq!(result.eval_as_string(), b"0");
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
            Err(ExprError::UnmatchedOpeningParenthesis)
        ));

        // Unmatched closing parenthesis
        let result = evaluate_match_expression(b"hello".to_vec(), b"hello\\)".to_vec());
        assert!(matches!(
            result,
            Err(ExprError::UnmatchedClosingParenthesis)
        ));

        // Trailing backslash
        let result = evaluate_match_expression(b"hello".to_vec(), b"hello\\".to_vec());
        assert!(matches!(result, Err(ExprError::TrailingBackslash)));

        // Invalid bracket content
        let result = evaluate_match_expression(b"hello".to_vec(), b"a\\{invalid\\}".to_vec());
        assert!(matches!(result, Err(ExprError::InvalidBracketContent)));
    }
}
