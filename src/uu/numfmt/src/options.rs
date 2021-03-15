use crate::units::Transform;
use uucore::ranges::Range;

pub const DELIMITER: &str = "delimiter";
pub const FIELD: &str = "field";
pub const FIELD_DEFAULT: &str = "1";
pub const FROM: &str = "from";
pub const FROM_DEFAULT: &str = "none";
pub const HEADER: &str = "header";
pub const HEADER_DEFAULT: &str = "1";
pub const NUMBER: &str = "NUMBER";
pub const PADDING: &str = "padding";
pub const TO: &str = "to";
pub const TO_DEFAULT: &str = "none";

pub struct TransformOptions {
    pub from: Transform,
    pub to: Transform,
}

pub struct NumfmtOptions {
    pub transform: TransformOptions,
    pub padding: isize,
    pub header: usize,
    pub fields: Vec<Range>,
    pub delimiter: Option<String>,
}
