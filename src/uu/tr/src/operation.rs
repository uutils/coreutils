//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (strings) anychar combinator Alnum Punct Xdigit alnum punct xdigit cntrl

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{anychar, digit1},
    combinator::{map, peek, value},
    multi::many0,
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
            Self::MissingCharClassName => writeln!(f, "missing character class name '[::]'"),
            Self::MissingEquivalentClassChar => {
                writeln!(f, "missing equivalence class character '[==]'")
            }
            Self::MultipleCharRepeatInSet2 => {
                writeln!(f, "only one [c*] repeat construct may appear in string2")
            }
            Self::CharRepeatInSet1 => {
                writeln!(f, "the [c*] repeat construct may not appear in string1")
            }
            Self::InvalidRepeatCount(count) => {
                writeln!(f, "invalid repeat count '{}' in [c*n] construct", count)
            }
            Self::EmptySet2WhenNotTruncatingSet1 => {
                writeln!(f, "when not truncating set1, string2 must be non-empty")
            }
        }
    }
}

impl Error for BadSequence {}
impl UError for BadSequence {}

#[derive(Debug, Clone, Copy)]
pub enum Sequence {
    Char(char),
    CharRange(u32, u32),
    CharStar(char),
    CharRepeat(char, usize),
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
    pub fn flatten(&self) -> Box<dyn Iterator<Item = char>> {
        match self {
            Self::Char(c) => Box::new(std::iter::once(*c)),
            Self::CharRange(l, r) => Box::new((*l..=*r).flat_map(std::char::from_u32)),
            Self::CharStar(c) => Box::new(std::iter::repeat(*c)),
            Self::CharRepeat(c, n) => Box::new(std::iter::repeat(*c).take(*n)),
            Self::Alnum => Box::new(('0'..='9').chain('A'..='Z').chain('a'..='z')),
            Self::Alpha => Box::new(('A'..='Z').chain('a'..='z')),
            Self::Blank => Box::new(unicode_table::BLANK.iter().cloned()),
            Self::Control => Box::new(
                (0..=31)
                    .chain(std::iter::once(127))
                    .flat_map(std::char::from_u32),
            ),
            Self::Digit => Box::new('0'..='9'),
            Self::Graph => Box::new(
                (48..=57) // digit
                    .chain(65..=90) // uppercase
                    .chain(97..=122) // lowercase
                    // punctuations
                    .chain(33..=47)
                    .chain(58..=64)
                    .chain(91..=96)
                    .chain(123..=126)
                    .chain(std::iter::once(32)) // space
                    .flat_map(std::char::from_u32),
            ),
            Self::Lower => Box::new('a'..='z'),
            Self::Print => Box::new(
                (48..=57) // digit
                    .chain(65..=90) // uppercase
                    .chain(97..=122) // lowercase
                    // punctuations
                    .chain(33..=47)
                    .chain(58..=64)
                    .chain(91..=96)
                    .chain(123..=126)
                    .flat_map(std::char::from_u32),
            ),
            Self::Punct => Box::new(
                (33..=47)
                    .chain(58..=64)
                    .chain(91..=96)
                    .chain(123..=126)
                    .flat_map(std::char::from_u32),
            ),
            Self::Space => Box::new(unicode_table::SPACES.iter().cloned()),
            Self::Upper => Box::new('A'..='Z'),
            Self::Xdigit => Box::new(('0'..='9').chain('A'..='F').chain('a'..='f')),
        }
    }

    // Hide all the nasty sh*t in here
    // TODO: Make the 2 set lazily generate the character mapping as necessary.
    pub fn solve_set_characters(
        set1_str: &str,
        set2_str: &str,
        truncate_set1_flag: bool,
    ) -> Result<(Vec<char>, Vec<char>), BadSequence> {
        let set1 = Self::from_str(set1_str)?;
        let set2 = Self::from_str(set2_str)?;

        let is_char_star = |s: &&Self| -> bool { matches!(s, Sequence::CharStar(_)) };
        let set1_star_count = set1.iter().filter(is_char_star).count();
        if set1_star_count == 0 {
            let set2_star_count = set2.iter().filter(is_char_star).count();
            if set2_star_count < 2 {
                let char_star = set2.iter().find_map(|s| match s {
                    Sequence::CharStar(c) => Some(c),
                    _ => None,
                });
                let mut partition = set2.as_slice().split(|s| matches!(s, Self::CharStar(_)));
                let set1_len = set1.iter().flat_map(Self::flatten).count();
                let set2_len = set2
                    .iter()
                    .filter_map(|s| match s {
                        Sequence::CharStar(_) => None,
                        r => Some(r),
                    })
                    .flat_map(Self::flatten)
                    .count();
                let star_compensate_len = set1_len.saturating_sub(set2_len);
                let (left, right) = (partition.next(), partition.next());
                let set2_solved: Vec<char> = match (left, right) {
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
                let mut set1_solved: Vec<char> = set1.iter().flat_map(Self::flatten).collect();
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
    pub fn from_str(input: &str) -> Result<Vec<Self>, BadSequence> {
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

    fn parse_backslash(input: &str) -> IResult<&str, char> {
        preceded(tag("\\"), anychar)(input).map(|(l, a)| {
            let c = match a {
                'a' => unicode_table::BEL,
                'b' => unicode_table::BS,
                'f' => unicode_table::FF,
                'n' => unicode_table::LF,
                'r' => unicode_table::CR,
                't' => unicode_table::HT,
                'v' => unicode_table::VT,
                x => x,
            };
            (l, c)
        })
    }

    fn parse_backslash_or_char(input: &str) -> IResult<&str, char> {
        alt((Self::parse_backslash, anychar))(input)
    }

    fn parse_char_range(input: &str) -> IResult<&str, Result<Self, BadSequence>> {
        separated_pair(
            Self::parse_backslash_or_char,
            tag("-"),
            Self::parse_backslash_or_char,
        )(input)
        .map(|(l, (a, b))| {
            (l, {
                let (start, end) = (u32::from(a), u32::from(b));
                Ok(Self::CharRange(start, end))
            })
        })
    }

    fn parse_char_star(input: &str) -> IResult<&str, Result<Self, BadSequence>> {
        delimited(tag("["), Self::parse_backslash_or_char, tag("*]"))(input)
            .map(|(l, a)| (l, Ok(Self::CharStar(a))))
    }

    fn parse_char_repeat(input: &str) -> IResult<&str, Result<Self, BadSequence>> {
        delimited(
            tag("["),
            separated_pair(Self::parse_backslash_or_char, tag("*"), digit1),
            tag("]"),
        )(input)
        .map(|(l, (c, cnt_str))| {
            let result = if cnt_str.starts_with('0') {
                match usize::from_str_radix(cnt_str, 8) {
                    Ok(0) => Ok(Self::CharStar(c)),
                    Ok(count) => Ok(Self::CharRepeat(c, count)),
                    Err(_) => Err(BadSequence::InvalidRepeatCount(cnt_str.to_string())),
                }
            } else {
                match cnt_str.parse::<usize>() {
                    Ok(0) => Ok(Self::CharStar(c)),
                    Ok(count) => Ok(Self::CharRepeat(c, count)),
                    Err(_) => Err(BadSequence::InvalidRepeatCount(cnt_str.to_string())),
                }
            };
            (l, result)
        })
    }

    fn parse_class(input: &str) -> IResult<&str, Result<Self, BadSequence>> {
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

    fn parse_char_equal(input: &str) -> IResult<&str, Result<Self, BadSequence>> {
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
    fn translate(&mut self, current: char) -> Option<char>;
}

#[derive(Debug)]
pub struct DeleteOperation {
    set: Vec<char>,
    complement_flag: bool,
}

impl DeleteOperation {
    pub fn new(set: Vec<char>, complement_flag: bool) -> Self {
        Self {
            set,
            complement_flag,
        }
    }
}

impl SymbolTranslator for DeleteOperation {
    fn translate(&mut self, current: char) -> Option<char> {
        let found = self.set.iter().any(|sequence| sequence.eq(&current));
        if self.complement_flag == found {
            Some(current)
        } else {
            None
        }
    }
}

pub struct TranslateOperationComplement {
    iter: u32,
    set2_iter: usize,
    set1: Vec<char>,
    set2: Vec<char>,
    translation_map: HashMap<char, char>,
}

impl TranslateOperationComplement {
    fn new(set1: Vec<char>, set2: Vec<char>) -> Self {
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
    translation_map: HashMap<char, char>,
}

impl TranslateOperationStandard {
    fn new(set1: Vec<char>, set2: Vec<char>) -> Result<Self, BadSequence> {
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
    fn next_complement_char(iter: u32, ignore_list: &[char]) -> (u32, char) {
        (iter..)
            .filter_map(std::char::from_u32)
            .filter(|c| !ignore_list.iter().any(|s| s.eq(c)))
            .map(|c| (u32::from(c) + 1, c))
            .next()
            .expect("exhausted all possible characters")
    }
}

impl TranslateOperation {
    pub fn new(set1: Vec<char>, set2: Vec<char>, complement: bool) -> Result<Self, BadSequence> {
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
    fn translate(&mut self, current: char) -> Option<char> {
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
    set1: HashSet<char>,
    complement: bool,
    previous: Option<char>,
}

impl SqueezeOperation {
    pub fn new(set1: Vec<char>, complement: bool) -> Self {
        Self {
            set1: set1.into_iter().collect(),
            complement,
            previous: None,
        }
    }
}

impl SymbolTranslator for SqueezeOperation {
    fn translate(&mut self, current: char) -> Option<char> {
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
    let mut buf = String::new();
    let mut output_buf = String::new();
    while let Ok(length) = input.read_line(&mut buf) {
        if length == 0 {
            break;
        } else {
            let filtered = buf.chars().filter_map(|c| translator.translate(c));
            output_buf.extend(filtered);
            output.write_all(output_buf.as_bytes()).unwrap();
        }
        buf.clear();
        output_buf.clear();
    }
}
