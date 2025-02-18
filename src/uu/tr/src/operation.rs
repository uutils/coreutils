// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (strings) anychar combinator Alnum Punct Xdigit alnum punct xdigit cntrl

use crate::unicode_table;
use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_till, take_until},
    character::complete::one_of,
    combinator::{map, map_opt, peek, recognize, value},
    multi::{many0, many_m_n},
    sequence::{delimited, preceded, separated_pair, terminated},
    IResult, Parser,
};
use std::{
    char,
    collections::{HashMap, HashSet},
    error::Error,
    fmt::{Debug, Display},
    io::{BufRead, Write},
    ops::Not,
};
use uucore::error::{UError, UResult, USimpleError};
use uucore::show_warning;

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
            Self::MissingCharClassName => write!(f, "missing character class name '[::]'"),
            Self::MissingEquivalentClassChar => {
                write!(f, "missing equivalence class character '[==]'")
            }
            Self::MultipleCharRepeatInSet2 => {
                write!(f, "only one [c*] repeat construct may appear in string2")
            }
            Self::CharRepeatInSet1 => {
                write!(f, "the [c*] repeat construct may not appear in string1")
            }
            Self::InvalidRepeatCount(count) => {
                write!(f, "invalid repeat count '{count}' in [c*n] construct")
            }
            Self::EmptySet2WhenNotTruncatingSet1 => {
                write!(f, "when not truncating set1, string2 must be non-empty")
            }
            Self::ClassExceptLowerUpperInSet2 => {
                write!(f, "when translating, the only character classes that may appear in set2 are 'upper' and 'lower'")
            }
            Self::ClassInSet2NotMatchedBySet1 => {
                write!(f, "when translating, every 'upper'/'lower' in set2 must be matched by a 'upper'/'lower' in the same position in set1")
            }
            Self::Set1LongerSet2EndsInClass => {
                write!(f, "when translating with string1 longer than string2,\nthe latter string must not end with a character class")
            }
            Self::ComplementMoreThanOneUniqueInSet2 => {
                write!(f, "when translating with complemented character classes,\nstring2 must map all characters in the domain to one")
            }
            Self::BackwardsRange { end, start } => {
                fn end_or_start_to_string(ut: &u32) -> String {
                    match char::from_u32(*ut) {
                        Some(ch @ '\x20'..='\x7E') => ch.escape_default().to_string(),
                        _ => {
                            format!("\\{ut:03o}")
                        }
                    }
                }

                write!(
                    f,
                    "range-endpoints of '{}-{}' are in reverse collating sequence order",
                    end_or_start_to_string(start),
                    end_or_start_to_string(end)
                )
            }
            Self::MultipleCharInEquivalence(s) => write!(
                f,
                "{s}: equivalence class operand must be a single character"
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
            Self::CharRepeat(c, n) => Box::new(std::iter::repeat(*c).take(*n)),
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
                    && !matches!(x, Self::Class(Class::Upper) | Self::Class(Class::Lower))
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
        set2_uniques.sort();
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

        if set2_solved.len() < set1_solved.len()
            && !truncate_set1_flag
            && matches!(
                set2.last().copied(),
                Some(Self::Class(Class::Upper)) | Some(Self::Class(Class::Lower))
            )
        {
            return Err(BadSequence::Set1LongerSet2EndsInClass);
        }
        //Truncation is done dead last. It has no influence on the other conversion steps
        if truncate_set1_flag {
            set1_solved.truncate(set2_solved.len());
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
                    let origin_octal: &str = std::str::from_utf8(input).unwrap();
                    let actual_octal_tail: &str = std::str::from_utf8(&input[0..2]).unwrap();
                    let outstand_char: char = char::from_u32(input[2] as u32).unwrap();
                    show_warning!(
                        "the ambiguous octal escape \\{} is being\n        interpreted as the 2-byte sequence \\0{}, {}",
                        origin_octal,
                        actual_octal_tail,
                        outstand_char
                    );
                }
                result
            },
        ).parse(input)
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
                    if v.is_empty() {
                        Ok(())
                    } else {
                        Err(v)
                    }
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
                        String::from_utf8_lossy(&[c]).into_owned(),
                        String::from_utf8_lossy(v).into_owned()
                    ))),
                },
            )
        })
    }
}

pub trait SymbolTranslator {
    fn translate(&mut self, current: u8) -> Option<u8>;

    /// Takes two SymbolTranslators and creates a new SymbolTranslator over both in sequence.
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

#[derive(Debug)]
pub struct DeleteOperation {
    set: Vec<u8>,
}

impl DeleteOperation {
    pub fn new(set: Vec<u8>) -> Self {
        Self { set }
    }
}

impl SymbolTranslator for DeleteOperation {
    fn translate(&mut self, current: u8) -> Option<u8> {
        // keep if not present in the set
        self.set.contains(&current).not().then_some(current)
    }
}

#[derive(Debug)]
pub struct TranslateOperation {
    translation_map: HashMap<u8, u8>,
}

impl TranslateOperation {
    pub fn new(set1: Vec<u8>, set2: Vec<u8>) -> Result<Self, BadSequence> {
        if let Some(fallback) = set2.last().copied() {
            Ok(Self {
                translation_map: set1
                    .into_iter()
                    .zip(set2.into_iter().chain(std::iter::repeat(fallback)))
                    .collect::<HashMap<_, _>>(),
            })
        } else if set1.is_empty() && set2.is_empty() {
            Ok(Self {
                translation_map: HashMap::new(),
            })
        } else {
            Err(BadSequence::EmptySet2WhenNotTruncatingSet1)
        }
    }
}

impl SymbolTranslator for TranslateOperation {
    fn translate(&mut self, current: u8) -> Option<u8> {
        Some(
            self.translation_map
                .get(&current)
                .copied()
                .unwrap_or(current),
        )
    }
}

#[derive(Debug, Clone)]
pub struct SqueezeOperation {
    set1: HashSet<u8>,
    previous: Option<u8>,
}

impl SqueezeOperation {
    pub fn new(set1: Vec<u8>) -> Self {
        Self {
            set1: set1.into_iter().collect(),
            previous: None,
        }
    }
}

impl SymbolTranslator for SqueezeOperation {
    fn translate(&mut self, current: u8) -> Option<u8> {
        let next = if self.set1.contains(&current) {
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
    let mut buf = Vec::new();
    let mut output_buf = Vec::new();

    while let Ok(length) = input.read_until(b'\n', &mut buf) {
        if length == 0 {
            break; // EOF reached
        }

        let filtered = buf.iter().filter_map(|&c| translator.translate(c));
        output_buf.extend(filtered);

        if let Err(e) = output.write_all(&output_buf) {
            return Err(USimpleError::new(
                1,
                format!("{}: write error: {}", uucore::util_name(), e),
            ));
        }

        buf.clear();
        output_buf.clear();
    }

    Ok(())
}
