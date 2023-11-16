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
            Self::Unparsed(s) => s.parse().ok(),
            _ => None,
        }
    }
    
    pub fn get_i64(&self) -> Option<i64> {
        match self {
            Self::SignedInt(n) => Some(*n),
            Self::Unparsed(s) => s.parse().ok(),
            _ => None,
        }
    }
    
    pub fn get_f64(&self) -> Option<f64> {
        match self {
            Self::Float(n) => Some(*n),
            Self::Unparsed(s) => s.parse().ok(),
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