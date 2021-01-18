use std::io::BufWriter;
use std::io::Write;
/// Get a file writer
///
/// Unlike the unix version of this function, this _always_ returns
/// a file writer
pub fn instantiate_current_writer(
    _filter: &Option<String>,
    filename: &str,
) -> BufWriter<Box<dyn Write>> {
    BufWriter::new(Box::new(
        // write to the next file
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(std::path::Path::new(&filename))
            .unwrap(),
    ) as Box<dyn Write>)
}
