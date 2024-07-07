// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::path::Path;
use uucore::display::print_verbatim;
use uucore::error::{UResult, UUsageError};
use uucore::line_ending::LineEnding;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = crate::uu_app().try_get_matches_from(args)?;

    let line_ending = LineEnding::from_zero_flag(matches.get_flag(crate::options::ZERO));

    let dirnames: Vec<String> = matches
        .get_many::<String>(crate::options::DIR)
        .unwrap_or_default()
        .cloned()
        .collect();

    if dirnames.is_empty() {
        return Err(UUsageError::new(1, "missing operand"));
    } else {
        for path in &dirnames {
            let p = Path::new(path);
            match p.parent() {
                Some(d) => {
                    if d.components().next().is_none() {
                        print!(".");
                    } else {
                        print_verbatim(d).unwrap();
                    }
                }
                None => {
                    if p.is_absolute() || path == "/" {
                        print!("/");
                    } else {
                        print!(".");
                    }
                }
            }
            print!("{line_ending}");
        }
    }

    Ok(())
}
