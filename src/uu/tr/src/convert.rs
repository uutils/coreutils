use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{anychar, one_of},
    combinator::{map_opt, recognize},
    multi::{many0, many_m_n},
    sequence::preceded,
    IResult,
};

fn parse_octal(input: &str) -> IResult<&str, char> {
    map_opt(
        preceded(tag("\\"), recognize(many_m_n(1, 3, one_of("01234567")))),
        |out: &str| {
            u32::from_str_radix(out, 8)
                .map(char::from_u32)
                .ok()
                .flatten()
        },
    )(input)
}

pub fn reduce_octal_to_char(input: String) -> String {
    let result = many0(alt((parse_octal, anychar)))(input.as_str())
        .map(|(_, r)| r)
        .unwrap()
        .into_iter()
        .collect();
    result
}
