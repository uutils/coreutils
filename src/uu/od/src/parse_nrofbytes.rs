pub fn parse_number_of_bytes(mut s: &str) -> Result<usize, &'static str> {
    let radix = if s.starts_with("0x") || s.starts_with("0X") {
        s = &s[2..];
        16
    } else if s.starts_with('0') {
        8
    } else {
        10
    };

    let (mut s, mut size_unit) = uucore::parse_size_unit(
        s,
        [
            &[b'k', b'K'][..],
            &[b'm', b'M'][..],
            &[b'G'][..],
            #[cfg(target_pointer_width = "64")]
            &[b'T'][..],
            #[cfg(target_pointer_width = "64")]
            &[b'P'][..],
            #[cfg(target_pointer_width = "64")]
            &[b'E'][..],
        ]
        .iter()
        .copied(),
        if radix != 16 { &[b'B'] } else { &[] },
    );

    if size_unit.is_empty() && radix != 16 {
        match s.as_bytes().last() {
            Some(b'b') => {
                size_unit.base = 512;
                size_unit.exp = 1;
                s = &s[..s.len() - 1];
            }
            Some(b'B') => return Err("parse failed"),
            _ => (),
        }
    }

    let i = usize::from_str_radix(s, radix).map_err("parse failed")?;
    Ok(i * size_unit.to_usize().unwrap())
}

#[allow(dead_code)]
fn parse_number_of_bytes_str(s: &str) -> Result<usize, &'static str> {
    parse_number_of_bytes(&String::from(s))
}

#[test]
fn test_parse_number_of_bytes() {
    // normal decimal numbers
    assert_eq!(0, parse_number_of_bytes_str("0").unwrap());
    assert_eq!(5, parse_number_of_bytes_str("5").unwrap());
    assert_eq!(999, parse_number_of_bytes_str("999").unwrap());
    assert_eq!(2 * 512, parse_number_of_bytes_str("2b").unwrap());
    assert_eq!(2 * 1024, parse_number_of_bytes_str("2k").unwrap());
    assert_eq!(4 * 1024, parse_number_of_bytes_str("4K").unwrap());
    assert_eq!(2 * 1048576, parse_number_of_bytes_str("2m").unwrap());
    assert_eq!(4 * 1048576, parse_number_of_bytes_str("4M").unwrap());
    assert_eq!(1073741824, parse_number_of_bytes_str("1G").unwrap());
    assert_eq!(2000, parse_number_of_bytes_str("2kB").unwrap());
    assert_eq!(4000, parse_number_of_bytes_str("4KB").unwrap());
    assert_eq!(2000000, parse_number_of_bytes_str("2mB").unwrap());
    assert_eq!(4000000, parse_number_of_bytes_str("4MB").unwrap());
    assert_eq!(2000000000, parse_number_of_bytes_str("2GB").unwrap());

    // octal input
    assert_eq!(8, parse_number_of_bytes_str("010").unwrap());
    assert_eq!(8 * 512, parse_number_of_bytes_str("010b").unwrap());
    assert_eq!(8 * 1024, parse_number_of_bytes_str("010k").unwrap());
    assert_eq!(8 * 1048576, parse_number_of_bytes_str("010m").unwrap());

    // hex input
    assert_eq!(15, parse_number_of_bytes_str("0xf").unwrap());
    assert_eq!(15, parse_number_of_bytes_str("0XF").unwrap());
    assert_eq!(27, parse_number_of_bytes_str("0x1b").unwrap());
    assert_eq!(16 * 1024, parse_number_of_bytes_str("0x10k").unwrap());
    assert_eq!(16 * 1048576, parse_number_of_bytes_str("0x10m").unwrap());

    // invalid input
    parse_number_of_bytes_str("").unwrap_err();
    parse_number_of_bytes_str("-1").unwrap_err();
    parse_number_of_bytes_str("1e2").unwrap_err();
    parse_number_of_bytes_str("xyz").unwrap_err();
    parse_number_of_bytes_str("b").unwrap_err();
    parse_number_of_bytes_str("1Y").unwrap_err();
    parse_number_of_bytes_str("∞").unwrap_err();
}

#[test]
#[cfg(target_pointer_width = "64")]
fn test_parse_number_of_bytes_64bits() {
    assert_eq!(1099511627776, parse_number_of_bytes_str("1T").unwrap());
    assert_eq!(1125899906842624, parse_number_of_bytes_str("1P").unwrap());
    assert_eq!(
        1152921504606846976,
        parse_number_of_bytes_str("1E").unwrap()
    );

    assert_eq!(2000000000000, parse_number_of_bytes_str("2TB").unwrap());
    assert_eq!(2000000000000000, parse_number_of_bytes_str("2PB").unwrap());
    assert_eq!(
        2000000000000000000,
        parse_number_of_bytes_str("2EB").unwrap()
    );
}
