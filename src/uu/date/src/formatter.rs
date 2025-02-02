use chrono::{Datelike, FixedOffset, NaiveDate, TimeZone, Timelike, Weekday};
use icu::datetime::time_zone::{FallbackFormat, IsoFormat, IsoMinutes, IsoSeconds, TimeZoneFormatter};
use icu::timezone::CustomTimeZone;
use core::fmt;
use std::str::FromStr;
use icu::calendar::DateTime;
use icu::datetime::options::components;
use icu::datetime::DateTimeFormatter;
use icu::locid::locale;
use std::collections::HashSet;
use chrono_tz::{OffsetName, Tz};

enum Padding {
    None,
    Space,
    Zero,
}

enum Case {
    Upper,
    Opposite,
    Original,
}

enum FormattedOutput {
    Numeric {
        value: i64,
        width: usize,
        padding: Padding,
    },
    Text {
        value: String,
        case: Case,
        width: usize,
        padding: Padding,
    },
}

impl Default for FormattedOutput {
    fn default() -> Self {
        FormattedOutput::Numeric {
            value: 0,
            width: 0,
            padding: Padding::None,
        }
    }
}

impl fmt::Display for FormattedOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormattedOutput::Numeric {
                value,
                width,
                padding,
            } => {
                let formatted = match padding {
                    Padding::Space => format!("{:>width$}", value, width = *width),
                    Padding::Zero => format!("{:0>width$}", value, width = *width),
                    Padding::None => format!("{}", value),
                };
                write!(f, "{}", formatted)
            }
            FormattedOutput::Text {
                value,
                case,
                width,
                padding,
            } => {
                let value = match case {
                    Case::Upper => value.to_uppercase(),
                    Case::Opposite => value.to_lowercase(),
                    Case::Original => value.to_string(),
                };
                let formatted = match padding {
                    Padding::Space => format!("{:>width$}", value, width = *width),
                    Padding::Zero => format!("{:0>width$}", value, width = *width),
                    Padding::None => value.to_string(),
                };

                write!(f, "{}", formatted)
            }
        }
    }
}

pub(crate) fn format(format_string: &str, datetime: chrono::DateTime<FixedOffset>, timezone: Option<Tz>) -> String {
    let mut formatted_result: String = "".to_string();

    let section_list = partition_format_string_into_sections(format_string);
    //println!("{:?}", section_list);
    for section in section_list {
        formatted_result = formatted_result + &format_section(&section, datetime, timezone);
    }
    formatted_result
}

fn format_section(section_string: &str, datetime: chrono::DateTime<FixedOffset>, timezone: Option<Tz>) -> String {
    // println!("section: {}", section_string);
    let locale = locale!("en_US");
    let mut section_chars = section_string.chars().peekable();
    let formatted_output: FormattedOutput;
    let mut case: Case = Case::Original;
    let mut padding: Option<Padding> = None;
    let mut _width: Option<usize>;
    let format_specifiers = HashSet::from([
        'a', 'A', 'b', 'B', 'C', 'd', 'D', 'e', 'F', 'q', 'g', 'G', 'h', 'H', 'I', 'j', 'k', 'l',
        'm', 'M', 'n', 'N', 'p', 'P', 'q', 'r', 'R', 's', 'S', 't', 'T', 'u', 'U', 'V', 'w', 'W',
        'x', 'X', 'y', 'Y', 'z', 'Z', '%',
    ]);
    let format_modifiers = HashSet::from(['#', '-', '_', '^', '+', '0']);
    let mut width_string = "".to_string();
    let mut timezone_offset_level = 0;
    let naive_date =
        NaiveDate::from_ymd_opt(datetime.year(), datetime.month(), datetime.day()).unwrap();
    let date = DateTime::try_new_iso_datetime(
        datetime.year(),
        datetime.month().try_into().unwrap(),
        datetime.day().try_into().unwrap(),
        datetime.hour().try_into().unwrap(),
        datetime.minute().try_into().unwrap(),
        datetime.second().try_into().unwrap(),
    )
    .unwrap();
    let date = date.to_any();
    let mut formatted_result: String = "".into();
    let output: String;

    section_chars.next();
    while let Some(current_char) = section_chars.next_if(|&c| format_modifiers.contains(&c)) {
        match current_char {
            '#' => case = Case::Opposite,
            '-' => padding = Some(Padding::None),
            '_' => padding = Some(Padding::Space),
            '^' => case = Case::Upper,
            '0' => padding = Some(Padding::Zero),
            //TODO implement format modifier '+'
            _ => (),
        }
    }

    while let Some(current_char) = section_chars.next_if(|&c| c.is_ascii_digit()) {
        width_string = width_string + &current_char.to_string();
    }

    while let Some(current_char) = section_chars.next_if(|&c| c.eq(&':')) {
        timezone_offset_level += 1;
    }

    if section_chars.next_if_eq(&'z').is_some() && 3 >= timezone_offset_level && timezone_offset_level >= 0 {
        formatted_result += {
            let tzf = TimeZoneFormatter::try_new(&locale.into(), icu::datetime::time_zone::TimeZoneFormatterOptions::from(FallbackFormat::Iso8601(
                        if timezone_offset_level == 0 {IsoFormat::Basic} else {IsoFormat::Extended},
                        if timezone_offset_level == 1 || timezone_offset_level == 2 { IsoMinutes::Required} else { IsoMinutes::Optional },
                        if timezone_offset_level == 2 {IsoSeconds::Never} else {IsoSeconds::Optional},
                    )),).unwrap();
            let formatted_output = FormattedOutput::Text {
                value: tzf.format_to_string(&CustomTimeZone::from_str(datetime.offset().to_string().as_str()).unwrap()),
                width: width_string.parse().unwrap_or(0),
                padding: padding.unwrap_or(Padding::None),
                case,
            };
            output = formatted_output.to_string();
            &output
        }
    } else if let Some(current_char) = section_chars.next_if(|&c| format_specifiers.contains(&c)) {
        formatted_result += match current_char {
            '%' => {
                formatted_output = FormattedOutput::Text {
                    value: "%".to_string(),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case: Case::Original,
                };
                output = formatted_output.to_string();
                &output
            }
            'b' => {
                let mut bag = components::Bag::default();
                bag.month = Some(components::Month::Short);
                let options = icu::datetime::DateTimeFormatterOptions::Components(bag);
                let dtf = DateTimeFormatter::try_new_experimental(&locale.into(), options).unwrap();
                formatted_output = FormattedOutput::Text {
                    value: dtf.format(&date).unwrap().to_string(),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case,
                };
                output = formatted_output.to_string();
                &output
            }
            'B' => {
                let mut bag = components::Bag::default();
                bag.month = Some(components::Month::Long);
                let options = icu::datetime::DateTimeFormatterOptions::Components(bag);
                let dtf = DateTimeFormatter::try_new_experimental(&locale.into(), options).unwrap();
                formatted_output = FormattedOutput::Text {
                    value: dtf.format(&date).unwrap().to_string(),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case,
                };
                output = formatted_output.to_string();
                &output
            }
            'a' => {
                let mut bag = components::Bag::default();
                bag.weekday = Some(components::Text::Short);
                let options = icu::datetime::DateTimeFormatterOptions::Components(bag);
                let dtf = DateTimeFormatter::try_new_experimental(&locale.into(), options).unwrap();
                formatted_output = FormattedOutput::Text {
                    value: dtf.format(&date).unwrap().to_string(),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case,
                };
                output = formatted_output.to_string();
                &output
            }
            'A' => {
                let mut bag = components::Bag::default();
                bag.weekday = Some(components::Text::Long);
                let options = icu::datetime::DateTimeFormatterOptions::Components(bag);
                let dtf = DateTimeFormatter::try_new_experimental(&locale.into(), options).unwrap();
                formatted_output = FormattedOutput::Text {
                    value: dtf.format(&date).unwrap().to_string(),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case,
                };
                output = formatted_output.to_string();
                &output
            }
            'C' => {
                formatted_output = FormattedOutput::Numeric {
                    value: (datetime.year() / 100).into(),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'd' => {
                formatted_output = FormattedOutput::Numeric {
                    value: (datetime.day()).into(),
                    width: width_string.parse().unwrap_or(2),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'D' => {
                formatted_output = FormattedOutput::Text {
                    value: format("%m/%d/%y", datetime, timezone),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case,
                };
                output = formatted_output.to_string();
                &output
            }
            'e' => {
                formatted_output = FormattedOutput::Numeric {
                    value: (datetime.day()).into(),
                    width: width_string.parse().unwrap_or(2),
                    padding: padding.unwrap_or(Padding::Space),
                };
                output = formatted_output.to_string();
                &output
            }
            'F' => {
                formatted_output = FormattedOutput::Text {
                    value: format("%+4Y-%m-%d", datetime, timezone),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case,
                };
                output = formatted_output.to_string();
                &output
            }
            'g' => {
                formatted_output = FormattedOutput::Numeric {
                    value: (naive_date.iso_week().year() % 100).into(),
                    width: width_string.parse().unwrap_or(2),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'G' => {
                formatted_output = FormattedOutput::Numeric {
                    value: naive_date.iso_week().year().into(),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'h' => {
                let mut bag = components::Bag::default();
                bag.month = Some(components::Month::Short);
                let options = icu::datetime::DateTimeFormatterOptions::Components(bag);
                let dtf = DateTimeFormatter::try_new_experimental(&locale.into(), options).unwrap();
                formatted_output = FormattedOutput::Text {
                    value: dtf.format(&date).unwrap().to_string(),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case,
                };
                output = formatted_output.to_string();
                &output
            }
            'H' => {
                formatted_output = FormattedOutput::Numeric {
                    value: (datetime.hour()).into(),
                    width: width_string.parse().unwrap_or(2),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'I' => {
                formatted_output = FormattedOutput::Numeric {
                    value: (datetime.hour12()).1.into(),
                    width: width_string.parse().unwrap_or(2),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'j' => {
                formatted_output = FormattedOutput::Numeric {
                    value: datetime.ordinal().into(),
                    width: width_string.parse().unwrap_or(2),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'k' => {
                formatted_output = FormattedOutput::Numeric {
                    value: (datetime.hour()).into(),
                    width: width_string.parse().unwrap_or(2),
                    padding: padding.unwrap_or(Padding::Space),
                };
                output = formatted_output.to_string();
                &output
            }
            'l' => {
                formatted_output = FormattedOutput::Numeric {
                    value: (datetime.hour12()).1.into(),
                    width: width_string.parse().unwrap_or(2),
                    padding: padding.unwrap_or(Padding::Space),
                };
                output = formatted_output.to_string();
                &output
            }
            'm' => {
                formatted_output = FormattedOutput::Numeric {
                    value: (datetime.month()).into(),
                    width: width_string.parse().unwrap_or(2),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'M' => {
                formatted_output = FormattedOutput::Numeric {
                    value: datetime.minute().into(),
                    width: width_string.parse().unwrap_or(2),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'n' => {
                formatted_output = FormattedOutput::Text {
                    value: "\n".to_string(),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case: Case::Original,
                };
                output = formatted_output.to_string();
                &output
            }
            'N' => {
                formatted_output = FormattedOutput::Numeric {
                    value: datetime.nanosecond().into(),
                    width: width_string.parse().unwrap_or(9),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'p' => {
                formatted_output = FormattedOutput::Text {
                    value: {
                        match datetime.hour12().0 {
                            false => "AM".to_owned(),
                            true => "PM".to_owned(),
                        }
                    },
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case,
                };
                output = formatted_output.to_string();
                &output
            }
            'P' => {
                formatted_output = FormattedOutput::Text {
                    value: {
                        match datetime.hour12().0 {
                            false => "am".to_owned(),
                            true => "pm".to_owned(),
                        }
                    },
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case,
                };
                output = formatted_output.to_string();
                &output
            }
            'q' => {
                formatted_output = FormattedOutput::Numeric {
                    value: ((datetime.month() / 3) + 1).into(),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'r' => {
                formatted_output = FormattedOutput::Text {
                    value: format("%l:%M:%S %p", datetime, timezone),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case,
                };
                output = formatted_output.to_string();
                &output
            }
            'R' => {
                formatted_output = FormattedOutput::Text {
                    value: format("%H:%M", datetime, timezone),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case,
                };
                output = formatted_output.to_string();
                &output
            }
            's' => {
                formatted_output = FormattedOutput::Numeric {
                    value: datetime.timestamp(),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'S' => {
                formatted_output = FormattedOutput::Numeric {
                    value: datetime.second().into(),
                    width: width_string.parse().unwrap_or(2),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            't' => {
                formatted_output = FormattedOutput::Text {
                    value: "\t".to_string(),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case: Case::Original,
                };
                output = formatted_output.to_string();
                &output
            }
            'T' => {
                formatted_output = FormattedOutput::Text {
                    value: format("%H:%M:%S", datetime, timezone),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case,
                };
                output = formatted_output.to_string();
                &output
            }
            'u' => {
                formatted_output = FormattedOutput::Numeric {
                    value: datetime.weekday().number_from_monday().into(),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'U' => {
                formatted_output = FormattedOutput::Numeric {
                    value: (naive_date.week(Weekday::Sun).first_day().ordinal0() / 7 + 1).into(),
                    width: width_string.parse().unwrap_or(2),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'V' => {
                formatted_output = FormattedOutput::Numeric {
                    value: naive_date.iso_week().week().into(),
                    width: width_string.parse().unwrap_or(2),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'W' => {
                formatted_output = FormattedOutput::Numeric {
                    value: (naive_date.week(Weekday::Mon).first_day().ordinal0() / 7 + 1).into(),
                    width: width_string.parse().unwrap_or(2),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'w' => {
                formatted_output = FormattedOutput::Numeric {
                    value: datetime.weekday().num_days_from_sunday().into(),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'x' => {
                formatted_output = FormattedOutput::Text {
                    value: format("%D", datetime, timezone),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case,
                };
                output = formatted_output.to_string();
                &output
            }
            'X' => {
                formatted_output = FormattedOutput::Text {
                    value: format("%T", datetime, timezone),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case,
                };
                output = formatted_output.to_string();
                &output
            }
            'y' => {
                formatted_output = FormattedOutput::Numeric {
                    value: (datetime.year() % 100).into(),
                    width: width_string.parse().unwrap_or(2),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'Y' => {
                formatted_output = FormattedOutput::Numeric {
                    value: datetime.year().into(),
                    width: width_string.parse().unwrap_or(2),
                    padding: padding.unwrap_or(Padding::Zero),
                };
                output = formatted_output.to_string();
                &output
            }
            'z' => {
                let tzf = TimeZoneFormatter::try_new(&locale.into(), icu::datetime::time_zone::TimeZoneFormatterOptions::from(FallbackFormat::Iso8601(
                            icu::datetime::time_zone::IsoFormat::Basic,
                            icu::datetime::time_zone::IsoMinutes::Required,
                            icu::datetime::time_zone::IsoSeconds::Optional,
                        )),).unwrap();
                formatted_output = FormattedOutput::Text {
                    value: tzf.format_to_string(&CustomTimeZone::from_str(datetime.offset().to_string().as_str()).unwrap()),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case,
                };
                output = formatted_output.to_string();
                &output
            }
            'Z' => {
                let offset = if let Some(tz) = timezone {
                    tz.offset_from_utc_datetime(&datetime.naive_utc())
                } else {
                    Tz::Etc__UTC.offset_from_utc_datetime(&datetime.naive_utc())
                };
                formatted_output = FormattedOutput::Text {
                    value: offset.abbreviation().unwrap_or("UTC").to_string(),
                    width: width_string.parse().unwrap_or(0),
                    padding: padding.unwrap_or(Padding::None),
                    case,
                };
                
                output = formatted_output.to_string();
                &output
            }
            _ => {
                output = "".to_string();
                &output
            }
        };
        //add remaining characters to result
        for c in section_chars {
            formatted_result = formatted_result + &c.to_string();
        }
    } else {
        formatted_output = FormattedOutput::Text {
            value: section_string.to_owned(),
            width: width_string.parse().unwrap_or(0),
            padding: padding.unwrap_or(Padding::Space),
            case: Case::Original,
        };
        formatted_result = formatted_result + &formatted_output.to_string();
    }
    // println!("{}", formatted_result);
    formatted_result
}

fn partition_format_string_into_sections(format_string: &str) -> Vec<String> {
    let chars = format_string.chars();
    let mut sections = Vec::new();
    let mut current_section = String::new();

    for c in chars {
        if c == '%' && current_section.len() == 1 {
            current_section.push(c);
        } else if c == '%' && !current_section.is_empty() {
            sections.push(current_section.clone());
            current_section.clear();
            current_section.push(c);
        } else {
            current_section.push(c);
        }
    }

    if !current_section.is_empty() {
        sections.push(current_section);
    }

    sections
}
