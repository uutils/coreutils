use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{anychar, digit1, one_of},
    combinator::{map_opt, recognize},
    multi::{many0, many_m_n},
    sequence::{delimited, preceded, separated_pair},
    IResult,
};
use std::{
    collections::HashMap,
    fmt::Debug,
    io::{BufRead, Write},
};

mod unicode_table {
    pub static BEL: char = '\u{0007}';
    pub static BS: char = '\u{0008}';
    pub static HT: char = '\u{0009}';
    pub static LF: char = '\u{000A}';
    pub static VT: char = '\u{000B}';
    pub static FF: char = '\u{000C}';
    pub static CR: char = '\u{000D}';
    pub static SPACE: char = '\u{0020}';
    pub static SPACES: &'static [char] = &[HT, LF, VT, FF, CR, SPACE];
    pub static BLANK: &'static [char] = &[SPACE, HT];
}

struct Repeat(char);

impl Repeat {
    fn new(element: char) -> Repeat {
        Repeat(element)
    }
}

impl Iterator for Repeat {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.0)
    }

    fn last(self) -> Option<Self::Item> {
        Some(self.0)
    }

    fn any<F>(&mut self, mut f: F) -> bool
    where
        Self: Sized,
        F: FnMut(Self::Item) -> bool,
    {
        f(self.0)
    }
}

fn truncate_iterator<T>(input: Option<usize>) -> impl Fn((usize, T)) -> Option<T> {
    move |(idx, c)| match input {
        Some(s) => match s.cmp(&idx) {
            std::cmp::Ordering::Greater => Some(c),
            _ => None,
        },
        None => Some(c),
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
            Sequence::CharStar(c) => Box::new(Repeat::new(*c)),
            Sequence::CharRepeat(c, n) => Box::new(Repeat::new(*c).take(*n)),
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

    pub fn last(&self) -> Option<char> {
        match self {
            Sequence::CharStar(c) => Some(*c),
            // TODO: Can be optimized further...
            rest => rest.flatten().last(),
        }
    }

    pub fn len(&self) -> Option<usize> {
        match self {
            Sequence::CharStar(_) => None,
            // TODO: Is there a fix for this?
            rest => Some(rest.flatten().count()),
        }
    }
}

impl Sequence {
    pub fn parse_set_string(input: &str) -> Vec<Sequence> {
        many0(alt((
            alt((
                Sequence::parse_char_range_octal_leftright,
                Sequence::parse_char_range_octal_left,
                Sequence::parse_char_range_octal_right,
                Sequence::parse_char_range_backslash_collapse,
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
                // NOTE: This must be the last one
            )),
            alt((
                Sequence::parse_octal,
                Sequence::parse_backslash,
                Sequence::parse_char,
            )),
        )))(input)
        .map(|(_, r)| r)
        .unwrap()
    }

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
                Sequence::CharRange(start, end)
            })
        })
    }

    fn parse_char_range_backslash_collapse(input: &str) -> IResult<&str, Sequence> {
        separated_pair(
            preceded(tag("\\"), anychar),
            tag("-"),
            preceded(tag("\\"), anychar),
        )(input)
        .map(|(l, (a, b))| {
            (l, {
                let (start, end) = (u32::from(a), u32::from(b));
                Sequence::CharRange(start, end)
            })
        })
    }

    fn parse_char_range_octal_left(input: &str) -> IResult<&str, Sequence> {
        separated_pair(
            preceded(tag("\\"), recognize(many_m_n(1, 3, one_of("01234567")))),
            tag("-"),
            anychar,
        )(input)
        .map(|(l, (a, b))| {
            (l, {
                let (start, end) = (u32::from_str_radix(a, 8).unwrap(), u32::from(b));
                Sequence::CharRange(start, end)
            })
        })
    }

    fn parse_char_range_octal_right(input: &str) -> IResult<&str, Sequence> {
        separated_pair(
            anychar,
            tag("-"),
            preceded(tag("\\"), recognize(many_m_n(1, 3, one_of("01234567")))),
        )(input)
        .map(|(l, (a, b))| {
            (l, {
                let (start, end) = (u32::from(a), u32::from_str_radix(b, 8).unwrap());
                Sequence::CharRange(start, end)
            })
        })
    }

    fn parse_char_range_octal_leftright(input: &str) -> IResult<&str, Sequence> {
        separated_pair(
            preceded(tag("\\"), recognize(many_m_n(1, 3, one_of("01234567")))),
            tag("-"),
            preceded(tag("\\"), recognize(many_m_n(1, 3, one_of("01234567")))),
        )(input)
        .map(|(l, (a, b))| {
            (l, {
                let (start, end) = (
                    u32::from_str_radix(a, 8).unwrap(),
                    u32::from_str_radix(b, 8).unwrap(),
                );
                Sequence::CharRange(start, end)
            })
        })
    }

    fn parse_char_star(input: &str) -> IResult<&str, Sequence> {
        delimited(tag("["), anychar, tag("*]"))(input).map(|(l, c)| (l, Sequence::CharStar(c)))
    }

    fn parse_char_repeat(input: &str) -> IResult<&str, Sequence> {
        delimited(
            tag("["),
            separated_pair(anychar, tag("*"), digit1),
            tag("]"),
        )(input)
        .map(|(l, (c, n))| (l, Sequence::CharRepeat(c, n.parse().unwrap())))
    }

    fn parse_alnum(input: &str) -> IResult<&str, Sequence> {
        tag("[:alnum:]")(input).map(|(l, _)| (l, Sequence::Alnum))
    }

    fn parse_alpha(input: &str) -> IResult<&str, Sequence> {
        tag("[:alpha:]")(input).map(|(l, _)| (l, Sequence::Alpha))
    }

    fn parse_blank(input: &str) -> IResult<&str, Sequence> {
        tag("[:blank:]")(input).map(|(l, _)| (l, Sequence::Blank))
    }

    fn parse_control(input: &str) -> IResult<&str, Sequence> {
        tag("[:cntrl:]")(input).map(|(l, _)| (l, Sequence::Control))
    }

    fn parse_digit(input: &str) -> IResult<&str, Sequence> {
        tag("[:digit:]")(input).map(|(l, _)| (l, Sequence::Digit))
    }

    fn parse_graph(input: &str) -> IResult<&str, Sequence> {
        tag("[:graph:]")(input).map(|(l, _)| (l, Sequence::Graph))
    }

    fn parse_lower(input: &str) -> IResult<&str, Sequence> {
        tag("[:lower:]")(input).map(|(l, _)| (l, Sequence::Lower))
    }

    fn parse_print(input: &str) -> IResult<&str, Sequence> {
        tag("[:print:]")(input).map(|(l, _)| (l, Sequence::Print))
    }

    fn parse_punct(input: &str) -> IResult<&str, Sequence> {
        tag("[:punct:]")(input).map(|(l, _)| (l, Sequence::Punct))
    }

    fn parse_space(input: &str) -> IResult<&str, Sequence> {
        tag("[:space:]")(input).map(|(l, _)| (l, Sequence::Space))
    }

    fn parse_upper(input: &str) -> IResult<&str, Sequence> {
        tag("[:upper:]")(input).map(|(l, _)| (l, Sequence::Upper))
    }

    fn parse_xdigit(input: &str) -> IResult<&str, Sequence> {
        tag("[:xdigit:]")(input).map(|(l, _)| (l, Sequence::Xdigit))
    }

    fn parse_char_equal(input: &str) -> IResult<&str, Sequence> {
        delimited(tag("[="), anychar, tag("=]"))(input).map(|(_, _)| todo!())
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
    pub fn new(set: Vec<Sequence>, complement_flag: bool) -> DeleteOperation {
        DeleteOperation {
            set: set.iter().flat_map(Sequence::flatten).collect::<Vec<_>>(),
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
    set1: Vec<char>,
    set2: Box<dyn Iterator<Item = char>>,
    fallback: char,
    translation_map: HashMap<char, char>,
}

impl TranslateOperationComplement {
    fn new(
        set1: Vec<Sequence>,
        set2: Vec<Sequence>,
        set1_truncate_length: Option<usize>,
        fallback: char,
    ) -> TranslateOperationComplement {
        TranslateOperationComplement {
            iter: 0,
            set1: set1
                .iter()
                .flat_map(Sequence::flatten)
                .enumerate()
                .filter_map(truncate_iterator(set1_truncate_length))
                .collect(),
            set2: Box::new(set2.into_iter().flat_map(|c| Sequence::flatten(&c))),
            fallback,
            translation_map: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct TranslateOperationStandard {
    translation_map: HashMap<char, char>,
}

impl TranslateOperationStandard {
    fn new(
        set1: Vec<Sequence>,
        set2: Vec<Sequence>,
        set1_truncate_length: Option<usize>,
        fallback: char,
    ) -> TranslateOperationStandard {
        TranslateOperationStandard {
            translation_map: set1
                .iter()
                .flat_map(Sequence::flatten)
                .zip(
                    set2.iter()
                        .flat_map(Sequence::flatten)
                        .chain(Repeat(fallback)),
                )
                .enumerate()
                .filter_map(truncate_iterator(set1_truncate_length))
                .collect::<HashMap<_, _>>(),
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
        set1: Vec<Sequence>,
        set2: Vec<Sequence>,
        truncate_set1: bool,
        complement: bool,
    ) -> TranslateOperation {
        let fallback = set2
            .iter()
            .rev()
            .next()
            .map(Sequence::last)
            .flatten()
            .unwrap();
        let set1_truncate_length = if truncate_set1 {
            set2.iter()
                .map(Sequence::len)
                .reduce(|a, b| match (a, b) {
                    (Some(l), Some(r)) => Some(l + r),
                    _ => None,
                })
                .flatten()
        } else {
            None
        };
        if complement {
            TranslateOperation::Complement(TranslateOperationComplement::new(
                set1,
                set2,
                set1_truncate_length,
                fallback,
            ))
        } else {
            TranslateOperation::Standard(TranslateOperationStandard::new(
                set1,
                set2,
                set1_truncate_length,
                fallback,
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
                        if let Some(p) = set2.next() {
                            let (next_index, next_value) =
                                TranslateOperation::next_complement_char(*iter, &*set1);
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
    set1: Vec<char>,
    complement: bool,
    previous: Option<char>,
}

impl SqueezeOperation {
    pub fn new(set1: Vec<Sequence>, complement: bool) -> SqueezeOperation {
        SqueezeOperation {
            set1: set1.iter().flat_map(Sequence::flatten).collect(),
            complement,
            previous: None,
        }
    }
}

impl SymbolTranslator for SqueezeOperation {
    fn translate(&mut self, current: char) -> Option<char> {
        if self.complement {
            let next = if self.set1.iter().any(|c| c.eq(&current)) {
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
            let next = if self.set1.iter().any(|c| c.eq(&current)) {
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
                    Sequence::parse_set_string(format!("\\{}{}{}", a, b, c).as_str()).len() == 1
                );
            }
        }
    }
}
