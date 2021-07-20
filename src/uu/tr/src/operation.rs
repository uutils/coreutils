use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{anychar, one_of},
    combinator::{map_opt, recognize, value},
    multi::{many0, many_m_n},
    sequence::{preceded, separated_pair, tuple},
    IResult,
};
use std::{
    collections::HashMap,
    fmt::Debug,
    io::{BufRead, Write},
};

use crate::unicode_table;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Sequence {
    Char(char),
    CharRange(Vec<char>),
}

impl Sequence {
    pub fn parse_set_string(input: &str) -> Vec<Sequence> {
        many0(alt((
            alt((Sequence::parse_octal, Sequence::parse_backslash)),
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
                Sequence::parse_space,
                Sequence::parse_upper,
                Sequence::parse_xdigit,
                Sequence::parse_char_equal,
                // NOTE: This must be the last one
                Sequence::parse_char,
            )),
        )))(input)
        .map(|(_, r)| r)
        .unwrap()
    }

    pub fn dissolve(self) -> Vec<char> {
        match self {
            Sequence::Char(c) => vec![c],
            Sequence::CharRange(r) => r,
        }
    }

    /// Sequence parsers

    fn parse_char(input: &str) -> IResult<&str, Sequence> {
        anychar(input).map(|(l, r)| (l, Sequence::Char(r)))
    }

    fn parse_backslash(input: &str) -> IResult<&str, Sequence> {
        preceded(tag("\\"), anychar)(input).map(|(l, a)| {
            let c = match a {
                'a' => Sequence::Char(unicode_table::BEL),
                'b' => Sequence::Char(unicode_table::BS),
                'f' => Sequence::Char(unicode_table::FF),
                'n' => Sequence::Char(unicode_table::LF),
                'r' => Sequence::Char(unicode_table::CR),
                't' => Sequence::Char(unicode_table::HT),
                'v' => Sequence::Char(unicode_table::VT),
                x => Sequence::Char(x),
            };
            (l, c)
        })
    }

    fn parse_octal(input: &str) -> IResult<&str, Sequence> {
        map_opt(
            preceded(tag("\\"), recognize(many_m_n(1, 3, one_of("01234567")))),
            |out: &str| {
                u32::from_str_radix(out, 8)
                    .map(|u| Sequence::Char(char::from_u32(u).unwrap()))
                    .ok()
            },
        )(input)
    }

    fn parse_char_range(input: &str) -> IResult<&str, Sequence> {
        separated_pair(anychar, tag("-"), anychar)(input).map(|(l, (a, b))| {
            (l, {
                let (start, end) = (u32::from(a), u32::from(b));
                Sequence::CharRange((start..=end).filter_map(std::char::from_u32).collect())
            })
        })
    }

    fn parse_char_star(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("["), anychar, tag("*]")))(input).map(|(_, (_, _, _))| todo!())
    }

    fn parse_char_repeat(input: &str) -> IResult<&str, Sequence> {
        tuple((
            tag("["),
            anychar,
            tag("*"),
            // TODO: Extend this to support octal as well. Octal starts with 0.
            take_while1(|c: char| c.is_digit(10)),
            tag("]"),
        ))(input)
        .map(|(l, (_, c, _, n, _))| {
            (
                l,
                Sequence::CharRange(std::iter::repeat(c).take(n.parse().unwrap()).collect()),
            )
        })
    }

    fn parse_alnum(input: &str) -> IResult<&str, Sequence> {
        tag("[:alnum:]")(input).map(|(l, _)| {
            (
                l,
                Sequence::CharRange(('0'..='9').chain('A'..='Z').chain('a'..='z').collect()),
            )
        })
    }

    fn parse_alpha(input: &str) -> IResult<&str, Sequence> {
        value(
            Sequence::CharRange(('A'..='Z').chain('a'..='z').collect()),
            tag("[:alpha:]"),
        )(input)
    }

    fn parse_blank(input: &str) -> IResult<&str, Sequence> {
        value(
            Sequence::CharRange(vec![unicode_table::SPACE, unicode_table::HT]),
            tag("[:blank:]"),
        )(input)
    }

    fn parse_control(input: &str) -> IResult<&str, Sequence> {
        value(
            Sequence::CharRange(
                (0..=31)
                    .chain(std::iter::once(127))
                    .flat_map(char::from_u32)
                    .collect(),
            ),
            tag("[:cntrl:]"),
        )(input)
    }

    fn parse_digit(input: &str) -> IResult<&str, Sequence> {
        value(Sequence::CharRange(('0'..='9').collect()), tag("[:digit:]"))(input)
    }

    fn parse_graph(input: &str) -> IResult<&str, Sequence> {
        value(
            Sequence::CharRange(
                (48..=57) // digit
                    .chain(65..=90) // uppercase
                    .chain(97..=122) // lowercase
                    // punctuations
                    .chain(33..=47)
                    .chain(58..=64)
                    .chain(91..=96)
                    .chain(123..=126)
                    .flat_map(char::from_u32)
                    .collect(),
            ),
            tag("[:graph:]"),
        )(input)
    }

    fn parse_lower(input: &str) -> IResult<&str, Sequence> {
        value(Sequence::CharRange(('a'..='z').collect()), tag("[:lower:]"))(input)
    }

    fn parse_print(input: &str) -> IResult<&str, Sequence> {
        tag("[:print:]")(input).map(|(_, _)| todo!())
    }

    fn parse_punct(input: &str) -> IResult<&str, Sequence> {
        value(
            Sequence::CharRange(
                (33..=47)
                    .chain(58..=64)
                    .chain(91..=96)
                    .chain(123..=126)
                    .flat_map(char::from_u32)
                    .collect(),
            ),
            tag("[:punct:]"),
        )(input)
    }

    fn parse_space(input: &str) -> IResult<&str, Sequence> {
        value(
            Sequence::CharRange(vec![
                unicode_table::HT,
                unicode_table::LF,
                unicode_table::VT,
                unicode_table::FF,
                unicode_table::CR,
                unicode_table::SPACE,
            ]),
            tag("[:space:]"),
        )(input)
    }

    fn parse_upper(input: &str) -> IResult<&str, Sequence> {
        tag("[:upper:]")(input).map(|(l, _)| (l, Sequence::CharRange(('A'..='Z').collect())))
    }

    fn parse_xdigit(input: &str) -> IResult<&str, Sequence> {
        tag("[:xdigit:]")(input).map(|(l, _)| {
            (
                l,
                Sequence::CharRange(('0'..='9').chain('A'..='F').chain('a'..='f').collect()),
            )
        })
    }

    fn parse_char_equal(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("[="), anychar, tag("=]")))(input).map(|(_, (_, _, _))| todo!())
    }
}

pub trait SymbolTranslator {
    fn translate(&mut self, current: char) -> Option<char>;
}

#[derive(Debug, Clone)]
pub struct DeleteOperation {
    set: Vec<Sequence>,
    complement_flag: bool,
}

impl DeleteOperation {
    pub fn new(set: Vec<Sequence>, complement_flag: bool) -> DeleteOperation {
        DeleteOperation {
            set,
            complement_flag,
        }
    }
}

impl SymbolTranslator for DeleteOperation {
    fn translate(&mut self, current: char) -> Option<char> {
        let found = self.set.iter().any(|sequence| match sequence {
            Sequence::Char(c) => c.eq(&current),
            Sequence::CharRange(r) => r.iter().any(|c| c.eq(&current)),
        });
        (self.complement_flag == found).then(|| current)
    }
}

#[derive(Debug, Clone)]
pub struct TranslateOperationComplement {
    iter: u32,
    set1: Vec<char>,
    set2: Vec<char>,
    fallback: char,
    translation_map: HashMap<char, char>,
}

impl TranslateOperationComplement {
    fn new(set1: Vec<char>, set2: Vec<char>, fallback: char) -> TranslateOperationComplement {
        TranslateOperationComplement {
            iter: 0,
            set1,
            set2: set2.into_iter().rev().collect(),
            fallback,
            translation_map: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TranslateOperationStandard {
    translation_map: HashMap<char, char>,
}

impl TranslateOperationStandard {
    fn new(set1: Vec<char>, set2: Vec<char>, fallback: char) -> TranslateOperationStandard {
        TranslateOperationStandard {
            translation_map: set1
                .into_iter()
                .zip(set2.into_iter().chain(std::iter::repeat(fallback)))
                .collect::<HashMap<_, _>>(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TranslateOperation {
    Standard(TranslateOperationStandard),
    Complement(TranslateOperationComplement),
}

impl TranslateOperation {
    fn next_complement_char(mut iter: u32) -> (u32, char) {
        while char::from_u32(iter).is_none() {
            iter = iter.saturating_add(1)
        }
        (iter.saturating_add(1), char::from_u32(iter).unwrap())
    }
}

impl TranslateOperation {
    pub fn new(
        pset1: Vec<Sequence>,
        pset2: Vec<Sequence>,
        truncate_set1: bool,
        complement: bool,
    ) -> TranslateOperation {
        // TODO: Only some translation is acceptable i.e. uppercase/lowercase transform.
        let mut set1 = pset1
            .into_iter()
            .flat_map(Sequence::dissolve)
            .collect::<Vec<_>>();
        let set2 = pset2
            .into_iter()
            .flat_map(Sequence::dissolve)
            .collect::<Vec<_>>();
        let fallback = set2.last().cloned().unwrap();
        if truncate_set1 {
            set1.truncate(set2.len());
        }
        if complement {
            TranslateOperation::Complement(TranslateOperationComplement::new(set1, set2, fallback))
        } else {
            TranslateOperation::Standard(TranslateOperationStandard::new(set1, set2, fallback))
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
                set1,
                set2,
                fallback,
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
                        if let Some(p) = set2.pop() {
                            let (next_index, next_value) =
                                TranslateOperation::next_complement_char(*iter);
                            *iter = next_index;
                            translation_map.insert(next_value, p);
                        } else {
                            translation_map.insert(current, *fallback);
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
    squeeze_set: Vec<char>,
    complement: bool,
    previous: Option<char>,
}

impl SqueezeOperation {
    pub fn new(squeeze_set: Vec<Sequence>, complement: bool) -> SqueezeOperation {
        SqueezeOperation {
            squeeze_set: squeeze_set
                .into_iter()
                .flat_map(Sequence::dissolve)
                .collect(),
            complement,
            previous: None,
        }
    }
}

impl SymbolTranslator for SqueezeOperation {
    fn translate(&mut self, current: char) -> Option<char> {
        if self.complement {
            let next = if self.squeeze_set.iter().any(|c| c.eq(&current)) {
                Some(current)
            } else {
                match self.previous {
                    Some(v) => {
                        if v.eq(&current) {
                            None
                        } else {
                            self.previous = Some(current);
                            Some(current)
                        }
                    }
                    None => {
                        self.previous = Some(current);
                        Some(current)
                    }
                }
            };
            self.previous = Some(current);
            next
        } else {
            let next = if self.squeeze_set.iter().any(|c| c.eq(&current)) {
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
fn test_parse_char_range() {
    assert_eq!(Sequence::parse_set_string(""), vec![]);
    assert_eq!(
        Sequence::parse_set_string("a-z"),
        vec![Sequence::CharRange(vec![
            'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q',
            'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
        ])]
    );
    assert_eq!(
        Sequence::parse_set_string("a-zA-Z"),
        vec![
            Sequence::CharRange(vec![
                'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p',
                'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
            ]),
            Sequence::CharRange(vec![
                'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P',
                'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
            ])
        ]
    );
    assert_eq!(
        Sequence::parse_set_string(", ┬─┬"),
        vec![
            Sequence::Char(','),
            Sequence::Char(' '),
            Sequence::Char('┬'),
            Sequence::Char('─'),
            Sequence::Char('┬')
        ]
    );
}

#[test]
fn test_parse_octal() {
    for a in '0'..='7' {
        for b in '0'..='7' {
            for c in '0'..='7' {
                assert!(
                    Sequence::parse_set_string(format!("\\{}{}{}", a, b, c).as_str()).len() == 1
                );
            }
        }
    }
}
