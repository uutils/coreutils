// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (strings) anychar combinator Alnum Punct Xdigit alnum punct xdigit cntrl boop

use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    character::complete::{digit1, one_of},
    combinator::{map, map_opt, peek, recognize, value},
    multi::{many0, many_m_n},
    sequence::{delimited, preceded, separated_pair},
    IResult,
};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt::{Debug, Display},
    io::{BufRead, Write},
};
use uucore::error::UError;

use crate::unicode_table;

#[derive(Debug, Clone)]
pub enum BadSequence {
    MissingCharClassName,
    MissingEquivalentClassChar,
    MultipleCharRepeatInSet2,
    CharRepeatInSet1,
    InvalidRepeatCount(String),
    EmptySet2WhenNotTruncatingSet1,
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
        }
    }
}

impl Error for BadSequence {}
impl UError for BadSequence {}

#[derive(Debug, Clone, Copy)]
pub enum Sequence {
    Char(u8),
    CharRange(u8, u8),
    CharStar(u8),
    CharRepeat(u8, usize),
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

impl Sequence {
    pub fn flatten(&self) -> Box<dyn Iterator<Item = u8>> {
        match self {
            Self::Char(c) => Box::new(std::iter::once(*c)),
            Self::CharRange(l, r) => Box::new(*l..=*r),
            Self::CharStar(c) => Box::new(std::iter::repeat(*c)),
            Self::CharRepeat(c, n) => Box::new(std::iter::repeat(*c).take(*n)),
            Self::Alnum => Box::new((b'0'..=b'9').chain(b'A'..=b'Z').chain(b'a'..=b'z')),
            Self::Alpha => Box::new((b'A'..=b'Z').chain(b'a'..=b'z')),
            Self::Blank => Box::new(unicode_table::BLANK.iter().cloned()),
            Self::Control => Box::new((0..=31).chain(std::iter::once(127))),
            Self::Digit => Box::new(b'0'..=b'9'),
            Self::Graph => Box::new(
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
            Self::Lower => Box::new(b'a'..=b'z'),
            Self::Print => Box::new(
                (48..=57) // digit
                    .chain(65..=90) // uppercase
                    .chain(97..=122) // lowercase
                    // punctuations
                    .chain(33..=47)
                    .chain(58..=64)
                    .chain(91..=96)
                    .chain(123..=126),
            ),
            Self::Punct => Box::new((33..=47).chain(58..=64).chain(91..=96).chain(123..=126)),
            Self::Space => Box::new(unicode_table::SPACES.iter().cloned()),
            Self::Upper => Box::new(b'A'..=b'Z'),
            Self::Xdigit => Box::new((b'0'..=b'9').chain(b'A'..=b'F').chain(b'a'..=b'f')),
        }
    }

    // Hide all the nasty sh*t in here
    pub fn solve_set_characters(
        set1_str: &[u8],
        set2_str: &[u8],
        truncate_set1_flag: bool,
    ) -> Result<(Vec<u8>, Vec<u8>), BadSequence> {
        let set1 = Self::from_str(set1_str)?;

        let is_char_star = |s: &&Self| -> bool { matches!(s, Self::CharStar(_)) };
        let set1_star_count = set1.iter().filter(is_char_star).count();
        if set1_star_count == 0 {
            let set2 = Self::from_str(set2_str)?;
            let set2_star_count = set2.iter().filter(is_char_star).count();
            if set2_star_count < 2 {
                let char_star = set2.iter().find_map(|s| match s {
                    Self::CharStar(c) => Some(c),
                    _ => None,
                });
                let mut partition = set2.as_slice().split(|s| matches!(s, Self::CharStar(_)));
                let set1_len = set1.iter().flat_map(Self::flatten).count();
                let set2_len = set2
                    .iter()
                    .filter_map(|s| match s {
                        Self::CharStar(_) => None,
                        r => Some(r),
                    })
                    .flat_map(Self::flatten)
                    .count();
                let star_compensate_len = set1_len.saturating_sub(set2_len);
                let (left, right) = (partition.next(), partition.next());
                let set2_solved: Vec<_> = match (left, right) {
                    (None, None) => match char_star {
                        Some(c) => std::iter::repeat(*c).take(star_compensate_len).collect(),
                        None => std::iter::empty().collect(),
                    },
                    (None, Some(set2_b)) => {
                        if let Some(c) = char_star {
                            std::iter::repeat(*c)
                                .take(star_compensate_len)
                                .chain(set2_b.iter().flat_map(Self::flatten))
                                .collect()
                        } else {
                            set2_b.iter().flat_map(Self::flatten).collect()
                        }
                    }
                    (Some(set2_a), None) => match char_star {
                        Some(c) => set2_a
                            .iter()
                            .flat_map(Self::flatten)
                            .chain(std::iter::repeat(*c).take(star_compensate_len))
                            .collect(),
                        None => set2_a.iter().flat_map(Self::flatten).collect(),
                    },
                    (Some(set2_a), Some(set2_b)) => match char_star {
                        Some(c) => set2_a
                            .iter()
                            .flat_map(Self::flatten)
                            .chain(std::iter::repeat(*c).take(star_compensate_len))
                            .chain(set2_b.iter().flat_map(Self::flatten))
                            .collect(),
                        None => set2_a
                            .iter()
                            .chain(set2_b.iter())
                            .flat_map(Self::flatten)
                            .collect(),
                    },
                };
                let mut set1_solved: Vec<_> = set1.iter().flat_map(Self::flatten).collect();
                if truncate_set1_flag {
                    set1_solved.truncate(set2_solved.len());
                }
                Ok((set1_solved, set2_solved))
            } else {
                Err(BadSequence::MultipleCharRepeatInSet2)
            }
        } else {
            Err(BadSequence::CharRepeatInSet1)
        }
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
            map(Self::parse_backslash_or_char, |s| Ok(Self::Char(s))),
        )))(input)
        .map(|(_, r)| r)
        .unwrap()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
    }

    fn parse_octal(input: &[u8]) -> IResult<&[u8], u8> {
        map_opt(
            preceded(tag("\\"), recognize(many_m_n(1, 3, one_of("01234567")))),
            |out: &[u8]| u8::from_str_radix(std::str::from_utf8(out).expect("boop"), 8).ok(),
        )(input)
    }

    fn parse_backslash(input: &[u8]) -> IResult<&[u8], u8> {
        preceded(tag("\\"), Self::single_char)(input).map(|(l, a)| {
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
        alt((Self::parse_octal, Self::parse_backslash, Self::single_char))(input)
    }

    fn single_char(input: &[u8]) -> IResult<&[u8], u8> {
        take(1usize)(input).map(|(l, a)| (l, a[0]))
    }

    fn parse_char_range(input: &[u8]) -> IResult<&[u8], Result<Self, BadSequence>> {
        separated_pair(
            Self::parse_backslash_or_char,
            tag("-"),
            Self::parse_backslash_or_char,
        )(input)
        .map(|(l, (a, b))| {
            (l, {
                let (start, end) = (u32::from(a), u32::from(b));
                Ok(Self::CharRange(start as u8, end as u8))
            })
        })
    }

    fn parse_char_star(input: &[u8]) -> IResult<&[u8], Result<Self, BadSequence>> {
        delimited(tag("["), Self::parse_backslash_or_char, tag("*]"))(input)
            .map(|(l, a)| (l, Ok(Self::CharStar(a))))
    }

    fn parse_char_repeat(input: &[u8]) -> IResult<&[u8], Result<Self, BadSequence>> {
        delimited(
            tag("["),
            separated_pair(Self::parse_backslash_or_char, tag("*"), digit1),
            tag("]"),
        )(input)
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
                        value(Self::Alnum, tag("alnum")),
                        value(Self::Alpha, tag("alpha")),
                        value(Self::Blank, tag("blank")),
                        value(Self::Control, tag("cntrl")),
                        value(Self::Digit, tag("digit")),
                        value(Self::Graph, tag("graph")),
                        value(Self::Lower, tag("lower")),
                        value(Self::Print, tag("print")),
                        value(Self::Punct, tag("punct")),
                        value(Self::Space, tag("space")),
                        value(Self::Upper, tag("upper")),
                        value(Self::Xdigit, tag("xdigit")),
                    )),
                    Ok,
                ),
                value(Err(BadSequence::MissingCharClassName), tag("")),
            )),
            tag(":]"),
        )(input)
    }

    fn parse_char_equal(input: &[u8]) -> IResult<&[u8], Result<Self, BadSequence>> {
        delimited(
            tag("[="),
            alt((
                value(
                    Err(BadSequence::MissingEquivalentClassChar),
                    peek(tag("=]")),
                ),
                map(Self::parse_backslash_or_char, |c| Ok(Self::Char(c))),
            )),
            tag("=]"),
        )(input)
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
    complement_flag: bool,
}

impl DeleteOperation {
    pub fn new(set: Vec<u8>, complement_flag: bool) -> Self {
        Self {
            set,
            complement_flag,
        }
    }
}

impl SymbolTranslator for DeleteOperation {
    fn translate(&mut self, current: u8) -> Option<u8> {
        let found = self.set.iter().any(|sequence| *sequence == current);
        if self.complement_flag == found {
            Some(current)
        } else {
            None
        }
    }
}

pub struct TranslateOperationComplement {
    iter: u8,
    set2_iter: usize,
    set1: Vec<u8>,
    set2: Vec<u8>,
    translation_map: HashMap<u8, u8>,
}

impl TranslateOperationComplement {
    fn new(set1: Vec<u8>, set2: Vec<u8>) -> Self {
        Self {
            iter: 0,
            set2_iter: 0,
            set1,
            set2,
            translation_map: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct TranslateOperationStandard {
    translation_map: HashMap<u8, u8>,
}

impl TranslateOperationStandard {
    fn new(set1: Vec<u8>, set2: Vec<u8>) -> Result<Self, BadSequence> {
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

pub enum TranslateOperation {
    Standard(TranslateOperationStandard),
    Complement(TranslateOperationComplement),
}

impl TranslateOperation {
    fn next_complement_char(iter: u8, ignore_list: &[u8]) -> (u8, u8) {
        (iter..)
            .filter(|c| !ignore_list.iter().any(|s| s == c))
            .map(|c| (c + 1, c))
            .next()
            .expect("exhausted all possible characters")
    }
}

impl TranslateOperation {
    pub fn new(set1: Vec<u8>, set2: Vec<u8>, complement: bool) -> Result<Self, BadSequence> {
        if complement {
            Ok(Self::Complement(TranslateOperationComplement::new(
                set1, set2,
            )))
        } else {
            Ok(Self::Standard(TranslateOperationStandard::new(set1, set2)?))
        }
    }
}

impl SymbolTranslator for TranslateOperation {
    fn translate(&mut self, current: u8) -> Option<u8> {
        match self {
            Self::Standard(TranslateOperationStandard { translation_map }) => Some(
                translation_map
                    .iter()
                    .find_map(|(l, r)| if l.eq(&current) { Some(*r) } else { None })
                    .unwrap_or(current),
            ),
            Self::Complement(TranslateOperationComplement {
                iter,
                set2_iter,
                set1,
                set2,
                translation_map,
            }) => {
                // First, try to see if current char is already mapped
                // If so, return the mapped char
                // Else, pop from set2
                // If we popped something, map the next complement character to this value
                // If set2 is empty, we just map the current char directly to fallback --- to avoid looping unnecessarily
                if let Some(c) = set1.iter().find(|c| c.eq(&&current)) {
                    Some(*c)
                } else {
                    while translation_map.get(&current).is_none() {
                        if let Some(value) = set2.get(*set2_iter) {
                            let (next_iter, next_key) = Self::next_complement_char(*iter, &*set1);
                            *iter = next_iter;
                            *set2_iter = set2_iter.saturating_add(1);
                            translation_map.insert(next_key, *value);
                        } else {
                            translation_map.insert(current, *set2.last().unwrap());
                        }
                    }
                    Some(*translation_map.get(&current).unwrap())
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SqueezeOperation {
    set1: HashSet<u8>,
    complement: bool,
    previous: Option<u8>,
}

impl SqueezeOperation {
    pub fn new(set1: Vec<u8>, complement: bool) -> Self {
        Self {
            set1: set1.into_iter().collect(),
            complement,
            previous: None,
        }
    }
}

impl SymbolTranslator for SqueezeOperation {
    fn translate(&mut self, current: u8) -> Option<u8> {
        if self.complement {
            let next = if self.set1.contains(&current) {
                Some(current)
            } else {
                match self.previous {
                    Some(v) => {
                        if v.eq(&current) {
                            None
                        } else {
                            Some(current)
                        }
                    }
                    None => Some(current),
                }
            };
            self.previous = Some(current);
            next
        } else {
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
}

pub fn translate_input<T, R, W>(input: &mut R, output: &mut W, mut translator: T)
where
    T: SymbolTranslator,
    R: BufRead,
    W: Write,
{
    let mut buf = Vec::new();
    let mut output_buf = Vec::new();
    while let Ok(length) = input.read_until(b'\n', &mut buf) {
        if length == 0 {
            break;
        } else {
            let filtered = buf.iter().filter_map(|c| translator.translate(*c));
            output_buf.extend(filtered);
            output.write_all(&output_buf).unwrap();
        }
        buf.clear();
        output_buf.clear();
    }
}
