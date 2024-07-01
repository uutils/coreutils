// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
#![deny(missing_docs)]

//! A collection of functions to parse the markdown code of help files.
//!
//! The structure of the markdown code is assumed to be:
//!
//! # util name
//!
//! ```text
//! usage info
//! ```
//!
//! About text
//!
//! ## Section 1
//!
//! Some content
//!
//! ## Section 2
//!
//! Some content

const MARKDOWN_CODE_FENCES: &str = "```";

/// Parses the text between the first markdown code block and the next header, if any,
/// into an about string.
pub fn parse_about(content: &str) -> String {
    content
        .lines()
        .skip_while(|l| !l.starts_with(MARKDOWN_CODE_FENCES))
        .skip(1)
        .skip_while(|l| !l.starts_with(MARKDOWN_CODE_FENCES))
        .skip(1)
        .take_while(|l| !l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Parses the first markdown code block into a usage string
///
/// The code fences are removed and the name of the util is replaced
/// with `{}` so that it can be replaced with the appropriate name
/// at runtime.
pub fn parse_usage(content: &str) -> String {
    content
        .lines()
        .skip_while(|l| !l.starts_with(MARKDOWN_CODE_FENCES))
        .skip(1)
        .take_while(|l| !l.starts_with(MARKDOWN_CODE_FENCES))
        .map(|l| {
            // Replace the util name (assumed to be the first word) with "{}"
            // to be replaced with the runtime value later.
            if let Some((_util, args)) = l.split_once(' ') {
                format!("{{}} {args}\n")
            } else {
                "{}\n".to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_string()
}

/// Get a single section from content
///
/// The section must be a second level section (i.e. start with `##`).
pub fn parse_section(section: &str, content: &str) -> Option<String> {
    fn is_section_header(line: &str, section: &str) -> bool {
        line.strip_prefix("##")
            .map_or(false, |l| l.trim().to_lowercase() == section)
    }

    let section = &section.to_lowercase();

    // We cannot distinguish between an empty or non-existing section below,
    // so we do a quick test to check whether the section exists
    if content.lines().all(|l| !is_section_header(l, section)) {
        return None;
    }

    // Prefix includes space to allow processing of section with level 3-6 headers
    let section_header_prefix = "## ";

    Some(
        content
            .lines()
            .skip_while(|&l| !is_section_header(l, section))
            .skip(1)
            .take_while(|l| !l.starts_with(section_header_prefix))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_section() {
        let input = "\
            # ls\n\
            ## some section\n\
            This is some section\n\
            \n\
            ## ANOTHER SECTION
            This is the other section\n\
            with multiple lines\n";

        assert_eq!(
            parse_section("some section", input).unwrap(),
            "This is some section"
        );
        assert_eq!(
            parse_section("SOME SECTION", input).unwrap(),
            "This is some section"
        );
        assert_eq!(
            parse_section("another section", input).unwrap(),
            "This is the other section\nwith multiple lines"
        );
    }

    #[test]
    fn test_parse_section_with_sub_headers() {
        let input = "\
            # ls\n\
            ## after section\n\
            This is some section\n\
            \n\
            ### level 3 header\n\
            \n\
            Additional text under the section.\n\
            \n\
            #### level 4 header\n\
            \n\
            Yet another paragraph\n";

        assert_eq!(
            parse_section("after section", input).unwrap(),
            "This is some section\n\n\
            ### level 3 header\n\n\
            Additional text under the section.\n\n\
            #### level 4 header\n\n\
            Yet another paragraph"
        );
    }

    #[test]
    fn test_parse_non_existing_section() {
        let input = "\
            # ls\n\
            ## some section\n\
            This is some section\n\
            \n\
            ## ANOTHER SECTION
            This is the other section\n\
            with multiple lines\n";

        assert!(parse_section("non-existing section", input).is_none());
    }

    #[test]
    fn test_parse_usage() {
        let input = "\
            # ls\n\
            ```\n\
            ls -l\n\
            ```\n\
            ## some section\n\
            This is some section\n\
            \n\
            ## ANOTHER SECTION
            This is the other section\n\
            with multiple lines\n";

        assert_eq!(parse_usage(input), "{} -l");
    }

    #[test]
    fn test_parse_multi_line_usage() {
        let input = "\
            # ls\n\
            ```\n\
            ls -a\n\
            ls -b\n\
            ls -c\n\
            ```\n\
            ## some section\n\
            This is some section\n";

        assert_eq!(parse_usage(input), "{} -a\n{} -b\n{} -c");
    }

    #[test]
    fn test_parse_about() {
        let input = "\
            # ls\n\
            ```\n\
            ls -l\n\
            ```\n\
            \n\
            This is the about section\n\
            \n\
            ## some section\n\
            This is some section\n";

        assert_eq!(parse_about(input), "This is the about section");
    }

    #[test]
    fn test_parse_multi_line_about() {
        let input = "\
            # ls\n\
            ```\n\
            ls -l\n\
            ```\n\
            \n\
            about a\n\
            \n\
            about b\n\
            \n\
            ## some section\n\
            This is some section\n";

        assert_eq!(parse_about(input), "about a\n\nabout b");
    }
}
