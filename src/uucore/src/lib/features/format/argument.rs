#[derive(Clone, Debug)]
pub enum FormatArgument {
    Char(char),
    String(String),
    UnsignedInt(u64),
    SignedInt(i64),
    Float(f64),
    /// Special argument that gets coerced into the other variants
    Unparsed(String),
}

impl FormatArgument {
    pub fn get_char(&self) -> Option<char> {
        match self {
            Self::Char(c) => Some(*c),
            Self::Unparsed(s) => {
                let mut chars = s.chars();
                let Some(c) = chars.next() else {
                    return None;
                };
                let None = chars.next() else {
                    return None;
                };
                Some(c)
            }
            _ => None,
        }
    }

    pub fn get_u64(&self) -> Option<u64> {
        match self {
            Self::UnsignedInt(n) => Some(*n),
            Self::Unparsed(s) => {
                if let Some(s) = s.strip_prefix("0x") {
                    u64::from_str_radix(s, 16).ok()
                } else if let Some(s) = s.strip_prefix("0") {
                    u64::from_str_radix(s, 8).ok()
                } else if let Some(s) = s.strip_prefix('\'') {
                    Some(s.chars().next()? as u64)
                } else {
                    s.parse().ok()
                }
            }
            _ => None,
        }
    }

    pub fn get_i64(&self) -> Option<i64> {
        match self {
            Self::SignedInt(n) => Some(*n),
            Self::Unparsed(s) => {
                // For hex, we parse `u64` because we do not allow another
                // minus sign. We might need to do more precise parsing here.
                if let Some(s) = s.strip_prefix("-0x") {
                    Some(- (u64::from_str_radix(s, 16).ok()? as i64))
                } else if let Some(s) = s.strip_prefix("0x") {
                    Some(u64::from_str_radix(s, 16).ok()? as i64)
                } else if s.starts_with("-0") || s.starts_with('0') {
                    i64::from_str_radix(s, 8).ok()
                } else if let Some(s) = s.strip_prefix('\'') {
                    Some(s.chars().next()? as i64)
                } else {
                    s.parse().ok()
                }
            }
            _ => None,
        }
    }

    pub fn get_f64(&self) -> Option<f64> {
        match self {
            Self::Float(n) => Some(*n),
            Self::Unparsed(s) => {
                if s.starts_with("0x") || s.starts_with("-0x") {
                    unimplemented!("Hexadecimal floats are unimplemented!")
                } else if let Some(s) = s.strip_prefix('\'') {
                    Some(s.chars().next()? as u64 as f64)
                } else {
                    s.parse().ok()
                }
            }
            _ => None,
        }
    }

    pub fn get_str(&self) -> Option<&str> {
        match self {
            Self::Unparsed(s) | Self::String(s) => Some(s),
            _ => None,
        }
    }
}
