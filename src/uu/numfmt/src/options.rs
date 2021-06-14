use crate::units::Transform;
use uucore::ranges::Range;

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
