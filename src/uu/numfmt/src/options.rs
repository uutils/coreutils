use crate::units::Unit;
use uucore::ranges::Range;

pub const DELIMITER: &str = "delimiter";
pub const FIELD: &str = "field";
pub const FIELD_DEFAULT: &str = "1";
pub const FROM: &str = "from";
pub const FROM_DEFAULT: &str = "none";
pub const FROM_UNIT: &str = "from-unit";
pub const FROM_UNIT_DEFAULT: &str = "1";
pub const HEADER: &str = "header";
pub const HEADER_DEFAULT: &str = "1";
pub const NUMBER: &str = "NUMBER";
pub const PADDING: &str = "padding";
pub const ROUND: &str = "round";
pub const SUFFIX: &str = "suffix";
pub const TO: &str = "to";
pub const TO_DEFAULT: &str = "none";
pub const TO_UNIT: &str = "to-unit";
pub const TO_UNIT_DEFAULT: &str = "1";

pub struct TransformOptions {
    pub from: Unit,
    pub from_unit: usize,
    pub to: Unit,
    pub to_unit: usize,
}

pub struct NumfmtOptions {
    pub transform: TransformOptions,
    pub padding: isize,
    pub header: usize,
    pub fields: Vec<Range>,
    pub delimiter: Option<String>,
    pub round: RoundMethod,
    pub suffix: Option<String>,
}

#[derive(Clone, Copy)]
pub enum RoundMethod {
    Up,
    Down,
    FromZero,
    TowardsZero,
    Nearest,
}

impl RoundMethod {
    pub fn round(&self, f: f64) -> f64 {
        match self {
            Self::Up => f.ceil(),
            Self::Down => f.floor(),
            Self::FromZero => {
                if f < 0.0 {
                    f.floor()
                } else {
                    f.ceil()
                }
            }
            Self::TowardsZero => {
                if f < 0.0 {
                    f.ceil()
                } else {
                    f.floor()
                }
            }
            Self::Nearest => f.round(),
        }
    }
}
