use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_until},
    character::complete::{none_of, one_of},
    multi::many0,
    sequence::{separated_pair, tuple},
    IResult,
};
use std::{
    collections::HashMap,
    io::{BufRead, Write},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Sequence {
    Char(char),
    CharRange(Vec<char>),
}

impl Sequence {
    pub fn parse_set_string(input: &str) -> Vec<Sequence> {
        many0(alt((
            alt((
                Sequence::parse_3_octal,
                Sequence::parse_2_octal,
                Sequence::parse_1_octal,
                Sequence::parse_unrecognized_backslash,
                Sequence::parse_backslash,
                Sequence::parse_audible_bel,
                Sequence::parse_backspace,
                Sequence::parse_form_feed,
                Sequence::parse_newline,
                Sequence::parse_return,
                Sequence::parse_horizontal_tab,
                Sequence::parse_vertical_tab,
            )),
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
        take(1usize)(input).map(|(l, r)| (l, Sequence::Char(r.chars().next().unwrap())))
    }

    fn parse_unrecognized_backslash(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), none_of("01234567")))(input).map(|(l, (_, a))| {
            let c = match a {
                'a' => Sequence::Char('\u{0007}'),
                'b' => Sequence::Char('\u{0008}'),
                'f' => Sequence::Char('\u{000C}'),
                'n' => Sequence::Char('\u{000A}'),
                'r' => Sequence::Char('\u{000D}'),
                't' => Sequence::Char('\u{0009}'),
                'v' => Sequence::Char('\u{000B}'),
                _ => Sequence::Char(a),
            };
            (l, c)
        })
    }

    fn parse_1_octal(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), one_of("01234567")))(input).map(|(l, (_, a))| {
            (
                l,
                Sequence::Char(std::char::from_u32(a.to_digit(8).unwrap()).unwrap()),
            )
        })
    }

    fn parse_2_octal(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), one_of("01234567"), one_of("01234567")))(input).map(|(l, (_, a, b))| {
            (
                l,
                Sequence::Char(
                    std::char::from_u32(a.to_digit(8).unwrap() * 8 + b.to_digit(8).unwrap())
                        .unwrap(),
                ),
            )
        })
    }

    fn parse_3_octal(input: &str) -> IResult<&str, Sequence> {
        tuple((
            tag("\\"),
            one_of("01234567"),
            one_of("01234567"),
            one_of("01234567"),
        ))(input)
        .map(|(l, (_, a, b, c))| {
            (
                l,
                Sequence::Char(
                    // SAFETY: All the values from \000 to \777 is valid based on a test below...
                    std::char::from_u32(
                        a.to_digit(8).unwrap() * 8 * 8
                            + b.to_digit(8).unwrap() * 8
                            + c.to_digit(8).unwrap(),
                    )
                    .unwrap(),
                ),
            )
        })
    }

    fn parse_backslash(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), tag("\\")))(input).map(|(l, _)| (l, Sequence::Char('\\')))
    }

    fn parse_audible_bel(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), tag("a")))(input).map(|(l, _)| (l, Sequence::Char('\u{0007}')))
    }

    fn parse_backspace(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), tag("b")))(input).map(|(l, _)| (l, Sequence::Char('\u{0008}')))
    }

    fn parse_form_feed(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), tag("f")))(input).map(|(l, _)| (l, Sequence::Char('\u{000C}')))
    }

    fn parse_newline(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), tag("n")))(input).map(|(l, _)| (l, Sequence::Char('\u{000A}')))
    }

    fn parse_return(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), tag("r")))(input).map(|(l, _)| (l, Sequence::Char('\u{000D}')))
    }

    fn parse_horizontal_tab(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), tag("t")))(input).map(|(l, _)| (l, Sequence::Char('\u{0009}')))
    }

    fn parse_vertical_tab(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), tag("v")))(input).map(|(l, _)| (l, Sequence::Char('\u{000B}')))
    }

    fn parse_char_range(input: &str) -> IResult<&str, Sequence> {
        separated_pair(take(1usize), tag("-"), take(1usize))(input).map(|(l, (a, b))| {
            (l, {
                let (start, end) = (
                    u32::from(a.chars().next().unwrap()),
                    u32::from(b.chars().next().unwrap()),
                );
                if (48..=90).contains(&start) && (48..=90).contains(&end) && end > start {
                    Sequence::CharRange(
                        (start..=end)
                            .map(|c| std::char::from_u32(c).unwrap())
                            .collect(),
                    )
                } else {
                    Sequence::CharRange((start..=end).filter_map(std::char::from_u32).collect())
                }
            })
        })
    }

    fn parse_char_star(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("["), take(1usize), tag("*"), tag("]")))(input).map(|(_, (_, _, _, _))| todo!())
    }

    fn parse_char_repeat(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("["), take(1usize), tag("*"), take_until("]"), tag("]")))(input).map(
            |(l, (_, c, _, n, _))| {
                (
                    l,
                    Sequence::CharRange(
                        std::iter::repeat(c.chars().next().unwrap())
                            .take(n.parse().unwrap())
                            .collect(),
                    ),
                )
            },
        )
    }

    fn parse_alnum(input: &str) -> IResult<&str, Sequence> {
        tag("[:alnum:]")(input).map(|(l, _)| {
            (
                l,
                Sequence::CharRange(('a'..='z').chain('A'..'Z').chain('0'..'9').collect()),
            )
        })
    }

    fn parse_alpha(input: &str) -> IResult<&str, Sequence> {
        tag("[:alpha:]")(input).map(|(l, _)| {
            (
                l,
                Sequence::CharRange(('a'..='z').chain('A'..'Z').collect()),
            )
        })
    }

    fn parse_blank(input: &str) -> IResult<&str, Sequence> {
        tag("[:blank:]")(input).map(|(_, _)| todo!())
    }

    fn parse_control(input: &str) -> IResult<&str, Sequence> {
        tag("[:cntrl:]")(input).map(|(_, _)| todo!())
    }

    fn parse_digit(input: &str) -> IResult<&str, Sequence> {
        tag("[:digit:]")(input).map(|(l, _)| (l, Sequence::CharRange(('0'..='9').collect())))
    }

    fn parse_graph(input: &str) -> IResult<&str, Sequence> {
        tag("[:graph:]")(input).map(|(_, _)| todo!())
    }

    fn parse_lower(input: &str) -> IResult<&str, Sequence> {
        tag("[:lower:]")(input).map(|(l, _)| (l, Sequence::CharRange(('a'..='z').collect())))
    }

    fn parse_print(input: &str) -> IResult<&str, Sequence> {
        tag("[:print:]")(input).map(|(_, _)| todo!())
    }

    fn parse_punct(input: &str) -> IResult<&str, Sequence> {
        tag("[:punct:]")(input).map(|(_, _)| todo!())
    }

    fn parse_space(input: &str) -> IResult<&str, Sequence> {
        tag("[:space:]")(input).map(|(_, _)| todo!())
    }

    fn parse_upper(input: &str) -> IResult<&str, Sequence> {
        tag("[:upper:]")(input).map(|(l, _)| (l, Sequence::CharRange(('A'..='Z').collect())))
    }

    fn parse_xdigit(input: &str) -> IResult<&str, Sequence> {
        tag("[:xdigit:]")(input).map(|(_, _)| todo!())
    }

    fn parse_char_equal(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("[="), take(1usize), tag("=]")))(input).map(|(_, (_, _, _))| todo!())
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
pub enum TranslateOperation {
    Standard(HashMap<char, char>),
    Complement(
        // iter
        u32,
        // set 1
        Vec<char>,
        // set 2
        Vec<char>,
        // fallback
        char,
        // translation map
        HashMap<char, char>,
    ),
}

impl TranslateOperation {
    fn next_complement_char(mut iter: u32) -> (u32, char) {
        while char::from_u32(iter).is_none() {
            iter = iter.saturating_add(1)
        }
        (iter, char::from_u32(iter).unwrap())
    }
}

impl TranslateOperation {
    pub fn new(
        pset1: Vec<Sequence>,
        pset2: Vec<Sequence>,
        truncate_set1: bool,
        complement: bool,
    ) -> TranslateOperation {
        let mut set1 = pset1
            .into_iter()
            .flat_map(Sequence::dissolve)
            .collect::<Vec<_>>();
        let set2 = pset2
            .into_iter()
            .flat_map(Sequence::dissolve)
            .collect::<Vec<_>>();
        if truncate_set1 {
            set1.truncate(set2.len());
        }
        let fallback = set2.last().cloned().unwrap();
        if complement {
            TranslateOperation::Complement(
                0,
                set1,
                set2,
                // TODO: Check how `tr` actually handles this
                fallback,
                HashMap::new(),
            )
        } else {
            TranslateOperation::Standard(
                set1.into_iter()
                    .zip(set2.into_iter().chain(std::iter::repeat(fallback)))
                    .collect::<HashMap<_, _>>(),
            )
        }
    }
}

impl SymbolTranslator for TranslateOperation {
    fn translate(&mut self, current: char) -> Option<char> {
        match self {
            TranslateOperation::Standard(map) => Some(
                map.iter()
                    .find_map(|(l, r)| l.eq(&current).then(|| *r))
                    .unwrap_or(current),
            ),
            TranslateOperation::Complement(iter, set1, set2, fallback, mapped_characters) => {
                // First, try to see if current char is already mapped
                // If so, return the mapped char
                // Else, pop from set2
                // If we popped something, map the next complement character to this value
                // If set2 is empty, we just map the current char directly to fallback --- to avoid looping unnecessarily
                if let Some(c) = set1.iter().find(|c| c.eq(&&current)) {
                    Some(*c)
                } else {
                    while mapped_characters.get(&current).is_none() {
                        if let Some(p) = set2.pop() {
                            let (next_index, next_value) =
                                TranslateOperation::next_complement_char(*iter);
                            *iter = next_index;
                            mapped_characters.insert(next_value, p);
                        } else {
                            mapped_characters.insert(current, *fallback);
                        }
                    }
                    Some(*mapped_characters.get(&current).unwrap())
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
                    Some(v) => {
                        if v.eq(&current) {
                            None
                        } else {
                            Some(current)
                        }
                    }
                    None => Some(current),
                }
            } else {
                Some(current)
            };
            self.previous = Some(current);
            next
        }
    }
}

pub fn translate_input_new<T, R, W>(input: &mut R, output: &mut W, mut translator: T)
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
