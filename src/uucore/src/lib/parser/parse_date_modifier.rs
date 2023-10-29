use std::collections::HashMap;
use ParseError::{EndOfString, IntOverflow, InvalidUnit, NoUnitPresent, UnexpectedToken};
use ParseState::{ExpectChrono, ExpectNum};

/// Struct to parse a string such as '+0001 day 100years + 27 HOURS'
/// and return the parsed values as a `HashMap<ChronoUnit, i64>`.
///
/// Functionality is exposed via `DateModParser::parse(haystack)`
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use uucore::parse_date_modifier::{ChronoUnit, DateModParser};
///
/// let map: HashMap<ChronoUnit, i64> = DateModParser::parse("+0001 day 100years + 27 HOURS").unwrap();
/// let expected = HashMap::from([
///     (ChronoUnit::Day, 1),
///     (ChronoUnit::Year, 100),
///     (ChronoUnit::Hour, 27)
/// ]);
/// assert_eq!(map, expected);
/// ```
pub struct DateModParser<'a> {
    state: ParseState,
    cursor: usize,
    haystack: &'a [u8],
}

impl<'a> DateModParser<'a> {
    pub fn parse(haystack: &'a str) -> Result<HashMap<ChronoUnit, i64>, ParseError> {
        Self {
            state: ExpectNum,
            cursor: 0,
            haystack: haystack.as_bytes(),
        }
        ._parse()
    }

    #[allow(clippy::map_entry)]
    fn _parse(&mut self) -> Result<HashMap<ChronoUnit, i64>, ParseError> {
        let mut map = HashMap::new();
        if self.haystack.is_empty() {
            return Ok(map);
        }
        let mut curr_num = 0;
        while self.cursor < self.haystack.len() {
            match self.state {
                ExpectNum => match self.parse_num() {
                    Ok(num) => {
                        curr_num = num;
                        self.state = ExpectChrono;
                    }
                    Err(EndOfString) => {
                        return Ok(map);
                    }
                    Err(e) => {
                        return Err(e);
                    }
                },
                ExpectChrono => {
                    match self.parse_unit() {
                        Ok(chrono) => {
                            if map.contains_key(&chrono) {
                                *map.get_mut(&chrono).unwrap() += curr_num;
                            } else {
                                map.insert(chrono, curr_num);
                            }
                            self.state = ExpectNum;
                        }
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }
            }
        }
        Ok(map)
    }

    fn parse_num(&mut self) -> Result<i64, ParseError> {
        while self.cursor < self.haystack.len() && self.haystack[self.cursor].is_ascii_whitespace()
        {
            self.cursor += 1;
        }
        if self.cursor >= self.haystack.len() {
            return Err(EndOfString);
        }
        let bytes = &self.haystack[self.cursor..];
        if bytes[0] == b'+' || bytes[0] == b'-' || (bytes[0] > 47 && bytes[0] <= 57) {
            let mut nums = vec![bytes[0] as char];
            let mut i = 1;
            loop {
                if i >= bytes.len() {
                    break;
                }
                if let n @ 48..=57 = bytes[i] {
                    nums.push(n as char);
                    if bytes[i].is_ascii_whitespace() {
                        self.cursor += 1;
                        break;
                    }
                    self.cursor += 1;
                    i += 1;
                } else if bytes[i].is_ascii_whitespace() {
                    self.cursor += 1;
                    i += 1;
                    continue;
                } else {
                    self.cursor += 1;
                    break;
                }
            }
            let n_as_string = nums.iter().collect::<String>();
            n_as_string.parse::<i64>().map_err(|_| IntOverflow)
        } else {
            Err(UnexpectedToken)
        }
    }

    fn parse_unit(&mut self) -> Result<ChronoUnit, ParseError> {
        while self.cursor < self.haystack.len() && self.haystack[self.cursor].is_ascii_whitespace()
        {
            self.cursor += 1;
        }
        if self.cursor >= self.haystack.len() {
            return Err(NoUnitPresent);
        }
        let bytes = &self.haystack[self.cursor..].to_ascii_lowercase();
        match bytes[0] {
            b'd' => {
                if let Some(slice) = bytes.get(0..) {
                    if slice.starts_with(b"days") {
                        self.cursor += 4;
                        return Ok(ChronoUnit::Day);
                    } else if slice.starts_with(b"day") {
                        self.cursor += 3;
                        return Ok(ChronoUnit::Day);
                    }
                }
            }
            b'w' => {
                if let Some(slice) = bytes.get(0..) {
                    if slice.starts_with(b"weeks") {
                        self.cursor += 5;
                        return Ok(ChronoUnit::Week);
                    } else if slice.starts_with(b"week") {
                        self.cursor += 4;
                        return Ok(ChronoUnit::Week);
                    }
                }
            }
            b'm' => {
                if let Some(slice) = bytes.get(0..) {
                    if slice.starts_with(b"months") {
                        self.cursor += 6;
                        return Ok(ChronoUnit::Month);
                    } else if slice.starts_with(b"month") {
                        self.cursor += 5;
                        return Ok(ChronoUnit::Month);
                    } else if slice.starts_with(b"minutes") {
                        self.cursor += 7;
                        return Ok(ChronoUnit::Minute);
                    } else if slice.starts_with(b"minute") {
                        self.cursor += 6;
                        return Ok(ChronoUnit::Minute);
                    }
                }
            }
            b'y' => {
                if let Some(slice) = bytes.get(0..) {
                    if slice.starts_with(b"years") {
                        self.cursor += 5;
                        return Ok(ChronoUnit::Year);
                    } else if slice.starts_with(b"year") {
                        self.cursor += 4;
                        return Ok(ChronoUnit::Year);
                    }
                }
            }
            b'h' => {
                if let Some(slice) = bytes.get(0..) {
                    if slice.starts_with(b"hours") {
                        self.cursor += 5;
                        return Ok(ChronoUnit::Hour);
                    } else if slice.starts_with(b"hour") {
                        self.cursor += 4;
                        return Ok(ChronoUnit::Hour);
                    }
                }
            }
            b's' => {
                if let Some(slice) = bytes.get(0..) {
                    if slice.starts_with(b"seconds") {
                        self.cursor += 7;
                        return Ok(ChronoUnit::Second);
                    } else if slice.starts_with(b"second") {
                        self.cursor += 6;
                        return Ok(ChronoUnit::Second);
                    }
                }
            }
            _ => {
                return Err(InvalidUnit);
            }
        }
        Err(InvalidUnit)
    }
}
/// Enum to represent units of time.
#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone)]
pub enum ChronoUnit {
    Day,
    Week,
    Month,
    Year,
    Hour,
    Minute,
    Second,
}

#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone)]
enum ParseState {
    ExpectNum,
    ExpectChrono,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ParseError {
    UnexpectedToken,
    NoUnitPresent,
    EndOfString,
    InvalidUnit,
    IntOverflow,
}

#[cfg(test)]
mod tests {
    use super::{ChronoUnit, DateModParser};
    use std::collections::HashMap;

    const HAYSTACK_OK_0: &str = "-10 year-10month   +000011day    +10year";
    const HAYSTACK_OK_1: &str = "-1000yeAR10MONTH-1               day           ";
    const HAYSTACK_OK_2: &str = "-000000100MONTH-1               seconds           ";
    const HAYSTACK_OK_3: &str = "+1000SECONDS-1yearS+000111HOURs                ";
    const HAYSTACK_OK_4: &str = "+1000SECONDS-1yearS000420minuTES  ";
    const HAYSTACK_OK_5: &str = "1 Month";

    const HAYSTACK_ERR_0: &str = "-10 yearz-10month   +000011day    +10year";
    const HAYSTACK_ERR_1: &str = "-10o0yeAR10MONTH-1               day           ";
    const HAYSTACK_ERR_2: &str = "+1000SECONDS-1yearS+000111HURs                ";
    const HAYSTACK_ERR_3: &str =
        "+100000000000000000000000000000000000000000000000000000000SECONDS  ";
    const HAYSTACK_ERR_4: &str = "+100000";
    const HAYSTACK_ERR_5: &str = "years";
    const HAYSTACK_ERR_6: &str = "----";

    #[test]
    fn test_parse_ok() {
        let expected0 = HashMap::from([
            (ChronoUnit::Year, 0),
            (ChronoUnit::Day, 11),
            (ChronoUnit::Month, -10),
        ]);
        let test0 = DateModParser::parse(HAYSTACK_OK_0).unwrap();
        assert_eq!(expected0, test0);

        let expected1 = HashMap::from([
            (ChronoUnit::Year, -1000),
            (ChronoUnit::Day, -1),
            (ChronoUnit::Month, 10),
        ]);
        let test1 = DateModParser::parse(HAYSTACK_OK_1).unwrap();
        assert_eq!(expected1, test1);

        let expected2 = HashMap::from([(ChronoUnit::Second, -1), (ChronoUnit::Month, -100)]);
        let test2 = DateModParser::parse(HAYSTACK_OK_2).unwrap();
        assert_eq!(expected2, test2);

        let expected3 = HashMap::from([
            (ChronoUnit::Second, 1000),
            (ChronoUnit::Year, -1),
            (ChronoUnit::Hour, 111),
        ]);
        let test3 = DateModParser::parse(HAYSTACK_OK_3).unwrap();
        assert_eq!(expected3, test3);

        let expected4 = HashMap::from([
            (ChronoUnit::Second, 1000),
            (ChronoUnit::Year, -1),
            (ChronoUnit::Minute, 420),
        ]);
        let test4 = DateModParser::parse(HAYSTACK_OK_4).unwrap();
        assert_eq!(expected4, test4);

        let expected5 = HashMap::from([(ChronoUnit::Month, 1)]);
        let test5 = DateModParser::parse(HAYSTACK_OK_5).unwrap();
        assert_eq!(expected5, test5);

        let expected5 = HashMap::from([(ChronoUnit::Month, 1)]);
        let test5 = DateModParser::parse(HAYSTACK_OK_5).unwrap();
        assert_eq!(expected5, test5);
    }

    #[test]
    fn test_parse_err() {
        let test0 = DateModParser::parse(HAYSTACK_ERR_0);
        assert!(test0.is_err());

        let test1 = DateModParser::parse(HAYSTACK_ERR_1);
        assert!(test1.is_err());

        let test2 = DateModParser::parse(HAYSTACK_ERR_2);
        assert!(test2.is_err());

        let test3 = DateModParser::parse(HAYSTACK_ERR_3);
        assert!(test3.is_err());

        let test4 = DateModParser::parse(HAYSTACK_ERR_4);
        assert!(test4.is_err());

        let test5 = DateModParser::parse(HAYSTACK_ERR_5);
        assert!(test5.is_err());

        let test6 = DateModParser::parse(HAYSTACK_ERR_6);
        assert!(test6.is_err());
    }
}
