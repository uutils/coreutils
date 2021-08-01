use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{anychar, one_of},
    combinator::{map, recognize},
    multi::{many0, many1},
    sequence::{delimited, preceded, separated_pair},
    IResult,
};
use std::{
    collections::{HashMap, HashSet},
    fmt::{Debug, Display},
    io::{BufRead, Write},
};

use crate::unicode_table;

#[derive(Debug)]
pub enum BadSequence {
    MissingCharClassName,
    MissingEquivalentClassChar,
    MultipleCharRepeatInSet2,
    CharRepeatInSet1,
    InvalidRepeatCount(String),
}

impl Display for BadSequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BadSequence::MissingCharClassName => {
                writeln!(f, "missing character class name '[::]'")
            }
            BadSequence::MissingEquivalentClassChar => {
                writeln!(f, "missing equivalence class character '[==]'")
            }
            BadSequence::MultipleCharRepeatInSet2 => {
                writeln!(f, "only one [c*] repeat construct may appear in string2")
            }
            BadSequence::CharRepeatInSet1 => {
                writeln!(f, "the [c*] repeat construct may not appear in string1")
            }
            BadSequence::InvalidRepeatCount(count) => {
                writeln!(f, "invalid repeat count '{}' in [c*n] construct", count)
            }
        }
    }
}

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
            Sequence::Char(c) => Box::new(std::iter::once(*c)),
            Sequence::CharRange(l, r) => Box::new((*l..=*r).flat_map(char::from_u32)),
            Sequence::CharStar(c) => Box::new(std::iter::repeat(*c)),
            Sequence::CharRepeat(c, n) => Box::new(std::iter::repeat(*c).take(*n)),
            Sequence::Alnum => Box::new(('0'..='9').chain('A'..='Z').chain('a'..='z')),
            Sequence::Alpha => Box::new(('A'..='Z').chain('a'..='z')),
            Sequence::Blank => Box::new(unicode_table::BLANK.into_iter().cloned()),
            Sequence::Control => Box::new(
                (0..=31)
                    .chain(std::iter::once(127))
                    .flat_map(char::from_u32),
            ),
            Sequence::Digit => Box::new('0'..='9'),
            Sequence::Graph => Box::new(
                (48..=57) // digit
                    .chain(65..=90) // uppercase
                    .chain(97..=122) // lowercase
                    // punctuations
                    .chain(33..=47)
                    .chain(58..=64)
                    .chain(91..=96)
                    .chain(123..=126)
                    .chain(std::iter::once(32)) // space
                    .flat_map(char::from_u32),
            ),
            Sequence::Lower => Box::new('a'..='z'),
            Sequence::Print => Box::new(
                (48..=57) // digit
                    .chain(65..=90) // uppercase
                    .chain(97..=122) // lowercase
                    // punctuations
                    .chain(33..=47)
                    .chain(58..=64)
                    .chain(91..=96)
                    .chain(123..=126)
                    .flat_map(char::from_u32),
            ),
            Sequence::Punct => Box::new(
                (33..=47)
                    .chain(58..=64)
                    .chain(91..=96)
                    .chain(123..=126)
                    .flat_map(char::from_u32),
            ),
            Sequence::Space => Box::new(unicode_table::SPACES.into_iter().cloned()),
            Sequence::Upper => Box::new('A'..='Z'),
            Sequence::Xdigit => Box::new(('0'..='9').chain('A'..='F').chain('a'..='f')),
        }
    }

    // Hide all the nasty sh*t in here
    // TODO: Make the 2 set lazily generate the character mapping as necessary.
    pub fn solve_set_characters(
        set1_str: &str,
        set2_str: &str,
        truncate_set1_flag: bool,
    ) -> Result<(Vec<char>, Vec<char>), BadSequence> {
        let set1 = Sequence::from_str(set1_str)?;
        let set2 = Sequence::from_str(set2_str)?;

        let is_char_star = |s: &&Sequence| -> bool {
            match s {
                Sequence::CharStar(_) => true,
                _ => false,
            }
        };
        let set1_star_count = set1.iter().filter(is_char_star).count();
        if set1_star_count == 0 {
            let set2_star_count = set2.iter().filter(is_char_star).count();
            if set2_star_count < 2 {
                let char_star = set2.iter().find_map(|s| match s {
                    Sequence::CharStar(c) => Some(c),
                    _ => None,
                });
                let mut partition = set2.as_slice().split(|s| match s {
                    Sequence::CharStar(_) => true,
                    _ => false,
                });
                let set1_len = set1.iter().flat_map(Sequence::flatten).count();
                let set2_len = set2
                    .iter()
                    .filter_map(|s| match s {
                        Sequence::CharStar(_) => None,
                        r => Some(r),
                    })
                    .flat_map(Sequence::flatten)
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
                                .chain(set2_b.iter().flat_map(Sequence::flatten))
                                .collect()
                        } else {
                            set2_b.iter().flat_map(Sequence::flatten).collect()
                        }
                    }
                    (Some(set2_a), None) => match char_star {
                        Some(c) => set2_a
                            .iter()
                            .flat_map(Sequence::flatten)
                            .chain(std::iter::repeat(*c).take(star_compensate_len))
                            .collect(),
                        None => set2_a.iter().flat_map(Sequence::flatten).collect(),
                    },
                    (Some(set2_a), Some(set2_b)) => match char_star {
                        Some(c) => set2_a
                            .iter()
                            .flat_map(Sequence::flatten)
                            .chain(std::iter::repeat(*c).take(star_compensate_len))
                            .chain(set2_b.iter().flat_map(Sequence::flatten))
                            .collect(),
                        None => set2_a
                            .iter()
                            .chain(set2_b.iter())
                            .flat_map(Sequence::flatten)
                            .collect(),
                    },
                };
                let mut set1_solved: Vec<char> = set1.iter().flat_map(Sequence::flatten).collect();
                if truncate_set1_flag {
                    set1_solved.truncate(set2_solved.len());
                }
                return Ok((set1_solved, set2_solved));
            } else {
                Err(BadSequence::MultipleCharRepeatInSet2)
            }
        } else {
            Err(BadSequence::CharRepeatInSet1)
        }
    }
}

impl Sequence {
    pub fn from_str(input: &str) -> Result<Vec<Sequence>, BadSequence> {
        let result = many0(alt((
            alt((
                Sequence::parse_char_range,
                Sequence::parse_char_star,
                Sequence::parse_char_repeat,
            )),
            alt((
                Sequence::parse_alnum,
                Sequence::parse_alpha,
                Sequence::parse_blank,
                Sequence::parse_control,
                Sequence::parse_digit,
                Sequence::parse_graph,
                Sequence::parse_lower,
                Sequence::parse_print,
                Sequence::parse_punct,
                Sequence::parse_space,
                Sequence::parse_upper,
                Sequence::parse_xdigit,
                Sequence::parse_char_equal,
            )),
            // NOTE: Specific error cases
            alt((
                Sequence::error_parse_char_repeat,
                Sequence::error_parse_empty_bracket,
                Sequence::error_parse_empty_equivalant_char,
            )),
            // NOTE: This must be the last one
            map(Sequence::parse_backslash_or_char, |s| Ok(Sequence::Char(s))),
        )))(input)
        .map(|(_, r)| r)
        .unwrap()
        .into_iter()
        .collect::<Result<Vec<_>, _>>();
        result
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
        alt((Sequence::parse_backslash, anychar))(input)
    }

    fn parse_char_range(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        separated_pair(
            Sequence::parse_backslash_or_char,
            tag("-"),
            Sequence::parse_backslash_or_char,
        )(input)
        .map(|(l, (a, b))| {
            (l, {
                let (start, end) = (u32::from(a), u32::from(b));
                Ok(Sequence::CharRange(start, end))
            })
        })
    }

    fn parse_char_star(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        delimited(tag("["), Sequence::parse_backslash_or_char, tag("*]"))(input)
            .map(|(l, a)| (l, Ok(Sequence::CharStar(a))))
    }

    fn parse_char_repeat(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        delimited(
            tag("["),
            separated_pair(
                Sequence::parse_backslash_or_char,
                tag("*"),
                recognize(many1(one_of("01234567"))),
            ),
            tag("]"),
        )(input)
        .map(|(l, (c, str))| {
            (
                l,
                match usize::from_str_radix(str, 8)
                    .expect("This should not fail because we only parse against 0-7")
                {
                    0 => Ok(Sequence::CharStar(c)),
                    count => Ok(Sequence::CharRepeat(c, count)),
                },
            )
        })
    }

    fn parse_alnum(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        tag("[:alnum:]")(input).map(|(l, _)| (l, Ok(Sequence::Alnum)))
    }

    fn parse_alpha(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        tag("[:alpha:]")(input).map(|(l, _)| (l, Ok(Sequence::Alpha)))
    }

    fn parse_blank(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        tag("[:blank:]")(input).map(|(l, _)| (l, Ok(Sequence::Blank)))
    }

    fn parse_control(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        tag("[:cntrl:]")(input).map(|(l, _)| (l, Ok(Sequence::Control)))
    }

    fn parse_digit(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        tag("[:digit:]")(input).map(|(l, _)| (l, Ok(Sequence::Digit)))
    }

    fn parse_graph(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        tag("[:graph:]")(input).map(|(l, _)| (l, Ok(Sequence::Graph)))
    }

    fn parse_lower(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        tag("[:lower:]")(input).map(|(l, _)| (l, Ok(Sequence::Lower)))
    }

    fn parse_print(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        tag("[:print:]")(input).map(|(l, _)| (l, Ok(Sequence::Print)))
    }

    fn parse_punct(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        tag("[:punct:]")(input).map(|(l, _)| (l, Ok(Sequence::Punct)))
    }

    fn parse_space(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        tag("[:space:]")(input).map(|(l, _)| (l, Ok(Sequence::Space)))
    }

    fn parse_upper(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        tag("[:upper:]")(input).map(|(l, _)| (l, Ok(Sequence::Upper)))
    }

    fn parse_xdigit(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        tag("[:xdigit:]")(input).map(|(l, _)| (l, Ok(Sequence::Xdigit)))
    }

    fn parse_char_equal(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        delimited(tag("[="), Sequence::parse_backslash_or_char, tag("=]"))(input)
            .map(|(l, c)| (l, Ok(Sequence::Char(c))))
    }
}

impl Sequence {
    fn error_parse_char_repeat(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        delimited(
            tag("["),
            separated_pair(
                Sequence::parse_backslash_or_char,
                tag("*"),
                recognize(many1(one_of("0123456789"))),
            ),
            tag("]"),
        )(input)
        .map(|(l, (_, n))| (l, Err(BadSequence::InvalidRepeatCount(n.to_string()))))
    }

    fn error_parse_empty_bracket(input: &str) -> IResult<&str, Result<Sequence, BadSequence>> {
        tag("[::]")(input).map(|(l, _)| (l, Err(BadSequence::MissingCharClassName)))
    }

    fn error_parse_empty_equivalant_char(
        input: &str,
    ) -> IResult<&str, Result<Sequence, BadSequence>> {
        tag("[==]")(input).map(|(l, _)| (l, Err(BadSequence::MissingEquivalentClassChar)))
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
    pub fn new(set: Vec<char>, complement_flag: bool) -> DeleteOperation {
        DeleteOperation {
            set,
            complement_flag,
        }
    }
}

impl SymbolTranslator for DeleteOperation {
    fn translate(&mut self, current: char) -> Option<char> {
        let found = self.set.iter().any(|sequence| sequence.eq(&current));
        (self.complement_flag == found).then(|| current)
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
    fn new(set1: Vec<char>, set2: Vec<char>) -> TranslateOperationComplement {
        TranslateOperationComplement {
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
    fn new(set1: Vec<char>, set2: Vec<char>) -> Result<TranslateOperationStandard, String> {
        if let Some(fallback) = set2.last().map(|s| *s) {
            Ok(TranslateOperationStandard {
                translation_map: set1
                    .into_iter()
                    .zip(set2.into_iter().chain(std::iter::repeat(fallback)))
                    .collect::<HashMap<_, _>>(),
            })
        } else {
            if set1.is_empty() && set2.is_empty() {
                Ok(TranslateOperationStandard {
                    translation_map: HashMap::new(),
                })
            } else {
                Err("when not truncating set1, string2 must be non-empty".to_string())
            }
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
            .filter_map(char::from_u32)
            .filter(|c| !ignore_list.iter().any(|s| s.eq(c)))
            .map(|c| (u32::from(c) + 1, c))
            .next()
            .expect("exhausted all possible characters")
    }
}

impl TranslateOperation {
    pub fn new(
        set1: Vec<char>,
        set2: Vec<char>,
        complement: bool,
    ) -> Result<TranslateOperation, String> {
        if complement {
            Ok(TranslateOperation::Complement(
                TranslateOperationComplement::new(set1, set2),
            ))
        } else {
            Ok(TranslateOperation::Standard(
                TranslateOperationStandard::new(set1, set2)?,
            ))
        }
    }
}

impl SymbolTranslator for TranslateOperation {
    fn translate(&mut self, current: char) -> Option<char> {
        match self {
            TranslateOperation::Standard(TranslateOperationStandard { translation_map }) => Some(
                translation_map
                    .iter()
                    .find_map(|(l, r)| l.eq(&current).then(|| *r))
                    .unwrap_or(current),
            ),
            TranslateOperation::Complement(TranslateOperationComplement {
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
                            let (next_iter, next_key) =
                                TranslateOperation::next_complement_char(*iter, &*set1);
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
    pub fn new(set1: Vec<char>, complement: bool) -> SqueezeOperation {
        SqueezeOperation {
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

#[test]
fn test_parse_octal() {
    for a in '0'..='7' {
        for b in '0'..='7' {
            for c in '0'..='7' {
                assert!(
                    Sequence::from_str(format!("\\{}{}{}", a, b, c).as_str())
                        .unwrap()
                        .len()
                        == 1
                );
            }
        }
    }
}
