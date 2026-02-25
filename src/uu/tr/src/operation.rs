// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (strings) anychar combinator Alnum Punct Xdigit alnum punct xdigit cntrl

use crate::unicode_table;
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{tag, take, take_till, take_until},
    character::complete::one_of,
    combinator::{map, map_opt, peek, recognize, value},
    multi::{many_m_n, many0},
    sequence::{delimited, preceded, separated_pair, terminated},
};
use std::{
    char,
    error::Error,
    fmt::{Debug, Display},
    io::{BufRead, Write},
};
use uucore::error::{FromIo, UError, UResult};
use uucore::translate;

use uucore::show_warning;

/// Common trait for operations that can process chunks of data
pub trait ChunkProcessor {
    fn process_chunk(&self, input: &[u8], output: &mut Vec<u8>);
}

#[derive(Debug, Clone)]
pub enum BadSequence {
    MissingCharClassName,
    MissingEquivalentClassChar,
    MultipleCharRepeatInSet2,
    CharRepeatInSet1,
    InvalidRepeatCount(String),
    EmptySet2WhenNotTruncatingSet1,
    ClassExceptLowerUpperInSet2,
    ClassInSet2NotMatchedBySet1,
    Set1LongerSet2EndsInClass,
    ComplementMoreThanOneUniqueInSet2,
    BackwardsRange { end: u32, start: u32 },
    MultipleCharInEquivalence(String),
}

impl Display for BadSequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingCharClassName => {
                write!(f, "{}", translate!("tr-error-missing-char-class-name"))
            }
            Self::MissingEquivalentClassChar => {
                write!(
                    f,
                    "{}",
                    translate!("tr-error-missing-equivalence-class-char")
                )
            }
            Self::MultipleCharRepeatInSet2 => {
                write!(f, "{}", translate!("tr-error-multiple-char-repeat-in-set2"))
            }
            Self::CharRepeatInSet1 => {
                write!(f, "{}", translate!("tr-error-char-repeat-in-set1"))
            }
            Self::InvalidRepeatCount(count) => {
                write!(
                    f,
                    "{}",
                    translate!("tr-error-invalid-repeat-count", "count" => format!("'{count}'"))
                )
            }
            Self::EmptySet2WhenNotTruncatingSet1 => {
                write!(
                    f,
                    "{}",
                    translate!("tr-error-empty-set2-when-not-truncating")
                )
            }
            Self::ClassExceptLowerUpperInSet2 => {
                write!(
                    f,
                    "{}",
                    translate!("tr-error-class-except-lower-upper-in-set2")
                )
            }
            Self::ClassInSet2NotMatchedBySet1 => {
                write!(f, "{}", translate!("tr-error-class-in-set2-not-matched"))
            }
            Self::Set1LongerSet2EndsInClass => {
                write!(
                    f,
                    "{}",
                    translate!("tr-error-set1-longer-set2-ends-in-class")
                )
            }
            Self::ComplementMoreThanOneUniqueInSet2 => {
                write!(
                    f,
                    "{}",
                    translate!("tr-error-complement-more-than-one-unique")
                )
            }
            Self::BackwardsRange { end, start } => {
                fn end_or_start_to_string(ut: u32) -> String {
                    match char::from_u32(ut) {
                        Some(ch @ '\x20'..='\x7E') => ch.escape_default().to_string(),
                        _ => {
                            format!("\\{ut:03o}")
                        }
                    }
                }
                write!(
                    f,
                    "{}",
                    translate!("tr-error-backwards-range", "start" => end_or_start_to_string(*start), "end" => end_or_start_to_string(*end))
                )
            }
            Self::MultipleCharInEquivalence(s) => write!(
                f,
                "{}",
                translate!("tr-error-multiple-char-in-equivalence", "chars" => s.clone())
            ),
        }
    }
}

impl Error for BadSequence {}
impl UError for BadSequence {}

#[derive(Debug, Clone, Copy)]
pub enum Class {
    Alnum,
    Alpha,
    Blank,
    Control,
    Digit,
    Graph,
    Lower,
    Print,
    Punct,
    Space,
    Upper,
    Xdigit,
}

#[derive(Debug, Clone, Copy)]
pub enum Sequence {
    Char(u8),
    CharRange(u8, u8),
    CharStar(u8),
    CharRepeat(u8, usize),
    Class(Class),
}

impl Sequence {
    pub fn flatten(&self) -> Box<dyn Iterator<Item = u8>> {
        match self {
            Self::Char(c) => Box::new(std::iter::once(*c)),
            Self::CharRange(l, r) => Box::new(*l..=*r),
            Self::CharStar(c) => Box::new(std::iter::repeat(*c)),
            Self::CharRepeat(c, n) => Box::new(std::iter::repeat_n(*c, *n)),
            Self::Class(class) => match class {
                Class::Alnum => Box::new((b'0'..=b'9').chain(b'A'..=b'Z').chain(b'a'..=b'z')),
                Class::Alpha => Box::new((b'A'..=b'Z').chain(b'a'..=b'z')),
                Class::Blank => Box::new(unicode_table::BLANK.iter().copied()),
                Class::Control => Box::new((0..=31).chain(std::iter::once(127))),
                Class::Digit => Box::new(b'0'..=b'9'),
                Class::Graph => Box::new(
                    (48..=57) // digit
                        .chain(65..=90) // uppercase
                        .chain(97..=122) // lowercase
                        // punctuations
                        .chain(33..=47)
                        .chain(58..=64)
                        .chain(91..=96)
                        .chain(123..=126)
                        .chain(std::iter::once(32)), // space
                ),
                Class::Print => Box::new(
                    (48..=57) // digit
                        .chain(65..=90) // uppercase
                        .chain(97..=122) // lowercase
                        // punctuations
                        .chain(33..=47)
                        .chain(58..=64)
                        .chain(91..=96)
                        .chain(123..=126),
                ),
                Class::Punct => Box::new((33..=47).chain(58..=64).chain(91..=96).chain(123..=126)),
                Class::Space => Box::new(unicode_table::SPACES.iter().copied()),
                Class::Xdigit => Box::new((b'0'..=b'9').chain(b'A'..=b'F').chain(b'a'..=b'f')),
                Class::Lower => Box::new(b'a'..=b'z'),
                Class::Upper => Box::new(b'A'..=b'Z'),
            },
        }
    }

    // Hide all the nasty sh*t in here
    pub fn solve_set_characters(
        set1_str: &[u8],
        set2_str: &[u8],
        complement_flag: bool,
        truncate_set1_flag: bool,
        translating: bool,
    ) -> Result<(Vec<u8>, Vec<u8>), BadSequence> {
        let is_char_star = |s: &&Self| -> bool { matches!(s, Self::CharStar(_)) };

        let set1 = Self::from_str(set1_str)?;
        if set1.iter().filter(is_char_star).count() != 0 {
            return Err(BadSequence::CharRepeatInSet1);
        }

        let mut set2 = Self::from_str(set2_str)?;
        if set2.iter().filter(is_char_star).count() > 1 {
            return Err(BadSequence::MultipleCharRepeatInSet2);
        }

        if translating
            && set2.iter().any(|&x| {
                matches!(x, Self::Class(_))
                    && !matches!(x, Self::Class(Class::Upper | Class::Lower))
            })
        {
            return Err(BadSequence::ClassExceptLowerUpperInSet2);
        }

        let mut set1_solved: Vec<u8> = set1.iter().flat_map(Self::flatten).collect();
        if complement_flag {
            set1_solved = (0..=u8::MAX).filter(|x| !set1_solved.contains(x)).collect();
        }
        let set1_len = set1_solved.len();

        let set2_len = set2
            .iter()
            .filter_map(|s| match s {
                Self::CharStar(_) => None,
                r => Some(r),
            })
            .flat_map(Self::flatten)
            .count();

        let star_compensate_len = set1_len.saturating_sub(set2_len);
        //Replace CharStar with CharRepeat
        set2 = set2
            .iter()
            .filter_map(|s| match s {
                Self::CharStar(0) => None,
                Self::CharStar(c) => Some(Self::CharRepeat(*c, star_compensate_len)),
                r => Some(*r),
            })
            .collect();

        // For every upper/lower in set2, there must be an upper/lower in set1 at the same position. The position is calculated by expanding everything before the upper/lower in both sets
        for (set2_pos, set2_item) in set2.iter().enumerate() {
            if matches!(set2_item, Self::Class(_)) {
                let mut set2_part_solved_len = 0;
                if set2_pos >= 1 {
                    set2_part_solved_len =
                        set2.iter().take(set2_pos).flat_map(Self::flatten).count();
                }

                let mut class_matches = false;
                for (set1_pos, set1_item) in set1.iter().enumerate() {
                    if matches!(set1_item, Self::Class(_)) {
                        let mut set1_part_solved_len = 0;
                        if set1_pos >= 1 {
                            set1_part_solved_len =
                                set1.iter().take(set1_pos).flat_map(Self::flatten).count();
                        }

                        if set1_part_solved_len == set2_part_solved_len {
                            class_matches = true;
                            break;
                        }
                    }
                }

                if !class_matches {
                    return Err(BadSequence::ClassInSet2NotMatchedBySet1);
                }
            }
        }

        let set2_solved: Vec<_> = set2.iter().flat_map(Self::flatten).collect();

        // Calculate the set of unique characters in set2
        let mut set2_uniques = set2_solved.clone();
        set2_uniques.sort_unstable();
        set2_uniques.dedup();

        // If the complement flag is used in translate mode, only one unique
        // character may appear in set2. Validate this with the set of uniques
        // in set2 that we just generated.
        // Also, set2 must not overgrow set1, otherwise the mapping can't be 1:1.
        if set1.iter().any(|x| matches!(x, Self::Class(_)))
            && translating
            && complement_flag
            && (set2_uniques.len() > 1 || set2_solved.len() > set1_len)
        {
            return Err(BadSequence::ComplementMoreThanOneUniqueInSet2);
        }

        if set2_solved.len() < set1_solved.len() {
            if truncate_set1_flag {
                set1_solved.truncate(set2_solved.len());
            } else if matches!(
                set2.last().copied(),
                Some(Self::Class(Class::Upper | Class::Lower))
            ) {
                return Err(BadSequence::Set1LongerSet2EndsInClass);
            }
        }

        Ok((set1_solved, set2_solved))
    }
}

impl Sequence {
    pub fn from_str(input: &[u8]) -> Result<Vec<Self>, BadSequence> {
        many0(alt((
            Self::parse_char_range,
            Self::parse_char_star,
            Self::parse_char_repeat,
            Self::parse_class,
            Self::parse_char_equal,
            // NOTE: This must be the last one
            map(Self::parse_backslash_or_char_with_warning, |s| {
                Ok(Self::Char(s))
            }),
        )))
        .parse(input)
        .map(|(_, r)| r)
        .unwrap()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
    }

    fn parse_octal(input: &[u8]) -> IResult<&[u8], u8> {
        // For `parse_char_range`, `parse_char_star`, `parse_char_repeat`, `parse_char_equal`.
        // Because in these patterns, there's no ambiguous cases.
        preceded(tag("\\"), Self::parse_octal_up_to_three_digits).parse(input)
    }

    fn parse_octal_with_warning(input: &[u8]) -> IResult<&[u8], u8> {
        preceded(
            tag("\\"),
            alt((
                Self::parse_octal_up_to_three_digits_with_warning,
                // Fallback for if the three digit octal escape is greater than \377 (0xFF), and therefore can't be
                // parsed as as a byte
                // See test `test_multibyte_octal_sequence`
                Self::parse_octal_two_digits,
            )),
        )
        .parse(input)
    }

    fn parse_octal_up_to_three_digits(input: &[u8]) -> IResult<&[u8], u8> {
        map_opt(
            recognize(many_m_n(1, 3, one_of("01234567"))),
            |out: &[u8]| {
                let str_to_parse = std::str::from_utf8(out).unwrap();
                u8::from_str_radix(str_to_parse, 8).ok()
            },
        )
        .parse(input)
    }

    fn parse_octal_up_to_three_digits_with_warning(input: &[u8]) -> IResult<&[u8], u8> {
        map_opt(
            recognize(many_m_n(1, 3, one_of("01234567"))),
            |out: &[u8]| {
                let str_to_parse = std::str::from_utf8(out).unwrap();
                let result = u8::from_str_radix(str_to_parse, 8).ok();
                if result.is_none() {
                    if let Ok(origin_octal) = std::str::from_utf8(input) {
                        let actual_octal_tail: &str = std::str::from_utf8(&input[0..2]).unwrap();
                        let outstand_char: char = char::from_u32(input[2] as u32).unwrap();
                        show_warning!(
                            "{}",
                            translate!("tr-warning-ambiguous-octal-escape", "origin_octal" => origin_octal, "actual_octal_tail" => actual_octal_tail, "outstand_char" => outstand_char)
                        );
                    } else {
                        show_warning!("{}", translate!("tr-warning-invalid-utf8"));
                    }
                }
                result
            },
        )
        .parse(input)
    }

    fn parse_octal_two_digits(input: &[u8]) -> IResult<&[u8], u8> {
        map_opt(
            recognize(many_m_n(2, 2, one_of("01234567"))),
            |out: &[u8]| u8::from_str_radix(std::str::from_utf8(out).unwrap(), 8).ok(),
        )
        .parse(input)
    }

    fn parse_backslash(input: &[u8]) -> IResult<&[u8], u8> {
        preceded(tag("\\"), Self::single_char)
            .parse(input)
            .map(|(l, a)| {
                let c = match a {
                    b'a' => unicode_table::BEL,
                    b'b' => unicode_table::BS,
                    b'f' => unicode_table::FF,
                    b'n' => unicode_table::LF,
                    b'r' => unicode_table::CR,
                    b't' => unicode_table::HT,
                    b'v' => unicode_table::VT,
                    x => x,
                };
                (l, c)
            })
    }

    fn parse_backslash_or_char(input: &[u8]) -> IResult<&[u8], u8> {
        alt((Self::parse_octal, Self::parse_backslash, Self::single_char)).parse(input)
    }

    fn parse_backslash_or_char_with_warning(input: &[u8]) -> IResult<&[u8], u8> {
        alt((
            Self::parse_octal_with_warning,
            Self::parse_backslash,
            Self::single_char,
        ))
        .parse(input)
    }

    fn single_char(input: &[u8]) -> IResult<&[u8], u8> {
        take(1usize)(input).map(|(l, a)| (l, a[0]))
    }

    fn parse_char_range(input: &[u8]) -> IResult<&[u8], Result<Self, BadSequence>> {
        separated_pair(
            Self::parse_backslash_or_char,
            tag("-"),
            Self::parse_backslash_or_char,
        )
        .parse(input)
        .map(|(l, (a, b))| {
            (l, {
                let (start, end) = (u32::from(a), u32::from(b));

                let range = start..=end;

                if range.is_empty() {
                    Err(BadSequence::BackwardsRange { end, start })
                } else {
                    Ok(Self::CharRange(start as u8, end as u8))
                }
            })
        })
    }

    fn parse_char_star(input: &[u8]) -> IResult<&[u8], Result<Self, BadSequence>> {
        delimited(tag("["), Self::parse_backslash_or_char, tag("*]"))
            .parse(input)
            .map(|(l, a)| (l, Ok(Self::CharStar(a))))
    }

    fn parse_char_repeat(input: &[u8]) -> IResult<&[u8], Result<Self, BadSequence>> {
        delimited(
            tag("["),
            separated_pair(
                Self::parse_backslash_or_char,
                tag("*"),
                // TODO
                // Why are the opening and closing tags not sufficient?
                // Backslash check is a workaround for `check_against_gnu_tr_tests_repeat_bs_9`
                take_till(|ue| matches!(ue, b']' | b'\\')),
            ),
            tag("]"),
        )
        .parse(input)
        .map(|(l, (c, cnt_str))| {
            let s = String::from_utf8_lossy(cnt_str);
            let result = if cnt_str.starts_with(b"0") {
                match usize::from_str_radix(&s, 8) {
                    Ok(0) => Ok(Self::CharStar(c)),
                    Ok(count) => Ok(Self::CharRepeat(c, count)),
                    Err(_) => Err(BadSequence::InvalidRepeatCount(s.to_string())),
                }
            } else {
                match s.parse::<usize>() {
                    Ok(0) => Ok(Self::CharStar(c)),
                    Ok(count) => Ok(Self::CharRepeat(c, count)),
                    Err(_) => Err(BadSequence::InvalidRepeatCount(s.to_string())),
                }
            };
            (l, result)
        })
    }

    fn parse_class(input: &[u8]) -> IResult<&[u8], Result<Self, BadSequence>> {
        delimited(
            tag("[:"),
            alt((
                map(
                    alt((
                        value(Self::Class(Class::Alnum), tag("alnum")),
                        value(Self::Class(Class::Alpha), tag("alpha")),
                        value(Self::Class(Class::Blank), tag("blank")),
                        value(Self::Class(Class::Control), tag("cntrl")),
                        value(Self::Class(Class::Digit), tag("digit")),
                        value(Self::Class(Class::Graph), tag("graph")),
                        value(Self::Class(Class::Lower), tag("lower")),
                        value(Self::Class(Class::Print), tag("print")),
                        value(Self::Class(Class::Punct), tag("punct")),
                        value(Self::Class(Class::Space), tag("space")),
                        value(Self::Class(Class::Upper), tag("upper")),
                        value(Self::Class(Class::Xdigit), tag("xdigit")),
                    )),
                    Ok,
                ),
                value(Err(BadSequence::MissingCharClassName), tag("")),
            )),
            tag(":]"),
        )
        .parse(input)
    }

    fn parse_char_equal(input: &[u8]) -> IResult<&[u8], Result<Self, BadSequence>> {
        preceded(
            tag("[="),
            (
                alt((
                    value(Err(()), peek(tag("=]"))),
                    map(Self::parse_backslash_or_char, Ok),
                )),
                map(terminated(take_until("=]"), tag("=]")), |v: &[u8]| {
                    if v.is_empty() { Ok(()) } else { Err(v) }
                }),
            ),
        )
        .parse(input)
        .map(|(l, (a, b))| {
            (
                l,
                match (a, b) {
                    (Err(()), _) => Err(BadSequence::MissingEquivalentClassChar),
                    (Ok(c), Ok(())) => Ok(Self::Char(c)),
                    (Ok(c), Err(v)) => Err(BadSequence::MultipleCharInEquivalence(format!(
                        "{}{}",
                        String::from_utf8_lossy(&[c]),
                        String::from_utf8_lossy(v),
                    ))),
                },
            )
        })
    }
}

pub trait SymbolTranslator {
    fn translate(&mut self, current: u8) -> Option<u8>;

    /// Takes two [`SymbolTranslator`]s and creates a new [`SymbolTranslator`] over both in sequence.
    ///
    /// This behaves pretty much identical to [`Iterator::chain`].
    fn chain<T>(self, other: T) -> ChainedSymbolTranslator<Self, T>
    where
        Self: Sized,
    {
        ChainedSymbolTranslator::<Self, T> {
            stage_a: self,
            stage_b: other,
        }
    }
}

pub struct ChainedSymbolTranslator<A, B> {
    stage_a: A,
    stage_b: B,
}

impl<A: SymbolTranslator, B: SymbolTranslator> SymbolTranslator for ChainedSymbolTranslator<A, B> {
    fn translate(&mut self, current: u8) -> Option<u8> {
        self.stage_a
            .translate(current)
            .and_then(|c| self.stage_b.translate(c))
    }
}

/// Convert a set of bytes to a 256-element bitmap for O(1) lookup
fn set_to_bitmap(set: &[u8]) -> [bool; 256] {
    let mut bitmap = [false; 256];
    for &byte in set {
        bitmap[byte as usize] = true;
    }
    bitmap
}

#[derive(Debug)]
pub struct DeleteOperation {
    pub(crate) delete_table: [bool; 256],
}

impl DeleteOperation {
    pub fn new(set: Vec<u8>) -> Self {
        Self {
            delete_table: set_to_bitmap(&set),
        }
    }
}

impl SymbolTranslator for DeleteOperation {
    fn translate(&mut self, current: u8) -> Option<u8> {
        // keep if not present in the delete set
        (!self.delete_table[current as usize]).then_some(current)
    }
}

impl ChunkProcessor for DeleteOperation {
    fn process_chunk(&self, input: &[u8], output: &mut Vec<u8>) {
        use crate::simd::{find_single_change, process_single_delete};

        // Check if this is single character deletion
        if let Some((delete_char, _)) =
            find_single_change(&self.delete_table, |_, &should_delete| should_delete)
        {
            process_single_delete(input, output, delete_char);
        } else {
            // Standard deletion
            output.extend(
                input
                    .iter()
                    .filter(|&&b| !self.delete_table[b as usize])
                    .copied(),
            );
        }
    }
}

#[derive(Debug)]
pub struct TranslateOperation {
    pub(crate) translation_table: [u8; 256],
}

impl TranslateOperation {
    pub fn new(set1: Vec<u8>, set2: Vec<u8>) -> Result<Self, BadSequence> {
        // Initialize translation table with identity mapping
        let mut translation_table = std::array::from_fn(|i| i as u8);

        if let Some(fallback) = set2.last().copied() {
            // Apply translations from set1 to set2
            for (from, to) in set1
                .into_iter()
                .zip(set2.into_iter().chain(std::iter::repeat(fallback)))
            {
                translation_table[from as usize] = to;
            }

            Ok(Self { translation_table })
        } else if set1.is_empty() && set2.is_empty() {
            // Identity mapping for empty sets
            Ok(Self { translation_table })
        } else {
            Err(BadSequence::EmptySet2WhenNotTruncatingSet1)
        }
    }
}

impl SymbolTranslator for TranslateOperation {
    fn translate(&mut self, current: u8) -> Option<u8> {
        Some(self.translation_table[current as usize])
    }
}

impl ChunkProcessor for TranslateOperation {
    fn process_chunk(&self, input: &[u8], output: &mut Vec<u8>) {
        use crate::simd::{find_single_change, process_single_char_replace};

        // Check if this is a simple single-character translation
        if let Some((source, target)) =
            find_single_change(&self.translation_table, |i, &val| val != i as u8)
        {
            // Use SIMD-optimized single character replacement
            process_single_char_replace(input, output, source, target);
        } else {
            // Standard translation using table lookup
            output.extend(input.iter().map(|&b| self.translation_table[b as usize]));
        }
    }
}

#[derive(Debug, Clone)]
pub struct SqueezeOperation {
    squeeze_table: [bool; 256],
    previous: Option<u8>,
}

impl SqueezeOperation {
    pub fn new(set1: Vec<u8>) -> Self {
        Self {
            squeeze_table: set_to_bitmap(&set1),
            previous: None,
        }
    }
}

impl SymbolTranslator for SqueezeOperation {
    fn translate(&mut self, current: u8) -> Option<u8> {
        let next = if self.squeeze_table[current as usize] {
            match self.previous {
                Some(v) if v == current => None,
                _ => Some(current),
            }
        } else {
            Some(current)
        };
        self.previous = Some(current);
        next
    }
}

pub fn translate_input<T, R, W>(input: &mut R, output: &mut W, mut translator: T) -> UResult<()>
where
    T: SymbolTranslator,
    R: BufRead,
    W: Write,
{
    const BUFFER_SIZE: usize = 32768; // Large buffer for better throughput
    let mut buf = [0; BUFFER_SIZE];
    let mut output_buf = Vec::with_capacity(BUFFER_SIZE);

    loop {
        let length = match input.read(&mut buf[..]) {
            Ok(0) => break, // EOF reached
            Ok(len) => len,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.map_err_context(|| translate!("tr-error-read-error"))),
        };

        // Process the buffer and collect translated chars to output
        output_buf.clear();
        for &byte in &buf[..length] {
            if let Some(translated) = translator.translate(byte) {
                output_buf.push(translated);
            }
        }

        if !output_buf.is_empty() {
            crate::simd::write_output(output, &output_buf)?;
        }
    }

    Ok(())
}

/// Platform-specific flush operation
#[inline]
pub fn flush_output<W: Write>(output: &mut W) -> UResult<()> {
    #[cfg(not(target_os = "windows"))]
    return output
        .flush()
        .map_err_context(|| translate!("tr-error-write-error"));

    #[cfg(target_os = "windows")]
    match output.flush() {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => {
            std::process::exit(13);
        }
        Err(err) => Err(err.map_err_context(|| translate!("tr-error-write-error"))),
    }
}
