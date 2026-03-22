// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore mangen tldr mandoc uppercasing uppercased manpages DESTDIR

use std::{
    collections::HashMap,
    ffi::OsString,
    fs::File,
    io::{self, Read, Seek, Write},
    process,
};

use clap::{Arg, Command};
use clap_complete::Shell;
use clap_mangen::Man;
use fluent_syntax::ast::{Entry, Message, Pattern};
use fluent_syntax::parser;
use jiff::Zoned;
use regex::Regex;
use textwrap::{fill, indent, termwidth};
use zip::ZipArchive;

use coreutils::validation;
use uucore::Args;
use uucore::locale::get_message;

include!(concat!(env!("OUT_DIR"), "/uutils_map.rs"));

/// Post-process a generated manpage to fix mandoc lint issues
///
/// This function:
/// - Fixes the TH header by uppercasing command names and adding a proper date
/// - Removes trailing whitespace from all lines
/// - Fixes redundant .br paragraph macros that cause mandoc warnings
/// - Removes .br before empty lines to avoid "br before sp" warnings
/// - Removes .br after empty lines to avoid "br after sp" warnings
/// - Fixes escape sequences (e.g., \\\\0 to \\0) to avoid "undefined escape" warnings
fn post_process_manpage(manpage: String, date: &str) -> String {
    // Only match TH headers that have at least a command name on the same line
    // Use [ \t] instead of \s to avoid matching newlines
    // Use a date format that satisfies mandoc (YYYY-MM-DD)

    let th_regex = Regex::new(r"(?m)^\.TH[ \t]+([^ \t\n]+)(?:[ \t]+[^\n]*)?$").unwrap();
    let mut result = th_regex
        .replace_all(&manpage, |caps: &regex::Captures| {
            // Add date to satisfy mandoc - date must be quoted
            format!(".TH {} 1 \"{date}\"", caps[1].to_uppercase())
        })
        .to_string();

    // Process lines: remove trailing whitespace and fix .br issues in a single pass
    let lines: Vec<&str> = result.lines().map(str::trim_end).collect();
    let mut fixed_lines: Vec<&str> = Vec::with_capacity(lines.len());

    for i in 0..lines.len() {
        let line = lines[i];

        if line == ".br" {
            let preceded_by_empty_line = i > 0 && lines[i - 1].is_empty();
            let followed_by_empty_line = i + 1 < lines.len() && lines[i + 1].is_empty();
            let followed_by_br = i + 1 < lines.len() && lines[i + 1] == ".br";

            if preceded_by_empty_line || followed_by_empty_line || followed_by_br {
                // skip this ".br"
                continue;
            }
        }

        fixed_lines.push(line);
    }

    result = fixed_lines.join("\n");

    // Fix escape sequence issues
    // \\\\0 appears when trying to represent literal \0 string
    // In man pages, use \e for literal backslash
    result = result.replace("\\\\\\\\0", "\\e0");
    result = result.replace("\\\\0", "\\e0");

    if !result.ends_with('\n') {
        result.push('\n');
    }

    result
}

/// Print usage information for uudoc
fn usage<T: Args>(utils: &UtilityMap<T>) {
    println!("uudoc - Documentation generator for uutils coreutils");
    println!();
    println!("Usage: uudoc [command] [args]");
    println!();
    println!("Commands:");
    println!("  (no command)                   Generate mdbook documentation (default)");
    println!("  manpage <utility>              Generate manpage for a utility");
    println!("  completion <utility> <shell>   Generate shell completions for a utility");
    println!();
    println!("Available utilities:");
    let all_utilities = validation::get_all_utilities(utils);
    let display_list = all_utilities.join(", ");
    let width = std::cmp::min(termwidth(), 100) - 4 * 2;
    println!("{}", indent(&fill(&display_list, width), "    "));
}

/// Generates the coreutils app for the utility map
fn gen_coreutils_app<T: Args>(util_map: &UtilityMap<T>) -> Command {
    let mut command = Command::new("coreutils");
    for (name, (_, sub_app)) in util_map {
        // Recreate a small subcommand with only the relevant info
        // (name & short description)
        let about = sub_app()
            .get_about()
            .expect("Could not get the 'about'")
            .to_string();
        let sub_app = Command::new(name).about(about);
        command = command.subcommand(sub_app);
    }
    command
}

/// Generate the manpage for the utility in the first parameter
fn gen_manpage<T: Args>(
    tldr: &mut Option<ZipArchive<File>>,
    args: impl Iterator<Item = OsString>,
    util_map: &UtilityMap<T>,
) -> ! {
    uucore::set_utility_is_second_arg();
    let all_utilities = validation::get_all_utilities(util_map);

    let matches = Command::new("manpage")
        .about("Prints manpage to stdout")
        .arg(
            Arg::new("utility")
                .value_parser(clap::builder::PossibleValuesParser::new(&all_utilities))
                .required(true),
        )
        .get_matches_from(std::iter::once(OsString::from("manpage")).chain(args));

    let utility = matches.get_one::<String>("utility").unwrap();
    let command = if utility == "coreutils" {
        gen_coreutils_app(util_map)
    } else {
        validation::setup_localization_or_exit(utility);
        let mut cmd = util_map.get(utility).unwrap().1();
        cmd.set_bin_name(utility.clone());
        let mut cmd = cmd.display_name(utility);
        if let Some(zip) = tldr {
            if let Ok(examples) = write_zip_examples(zip, utility, false) {
                cmd = cmd.after_help(examples);
            }
        }
        cmd
    };

    // Generate the manpage to a buffer first so we can post-process it
    let mut buffer = Vec::new();
    let man = Man::new(command);
    man.render(&mut buffer).expect("Man page generation failed");

    // Convert to string for processing
    let manpage = String::from_utf8(buffer).expect("Invalid UTF-8 in manpage");

    // Post-process the manpage to fix mandoc lint issues
    let date = Zoned::now().strftime("%Y-%m-%d").to_string();
    let processed_manpage = post_process_manpage(manpage, &date);

    // Write the processed manpage to stdout
    io::stdout()
        .write_all(processed_manpage.as_bytes())
        .unwrap();
    io::stdout().flush().unwrap();
    process::exit(0);
}

/// Generate shell completions for the utility in the first parameter
fn gen_completions<T: Args>(args: impl Iterator<Item = OsString>, util_map: &UtilityMap<T>) -> ! {
    let all_utilities = validation::get_all_utilities(util_map);

    let matches = Command::new("completion")
        .about("Prints completions to stdout")
        .arg(
            Arg::new("utility")
                .value_parser(clap::builder::PossibleValuesParser::new(&all_utilities))
                .required(true),
        )
        .arg(
            Arg::new("shell")
                .value_parser(clap::builder::EnumValueParser::<Shell>::new())
                .required(true),
        )
        .get_matches_from(std::iter::once(OsString::from("completion")).chain(args));

    let utility = matches.get_one::<String>("utility").unwrap();
    let shell = *matches.get_one::<Shell>("shell").unwrap();

    let mut command = if utility == "coreutils" {
        gen_coreutils_app(util_map)
    } else {
        validation::setup_localization_or_exit(utility);
        util_map.get(utility).unwrap().1()
    };
    let bin_name = std::env::var("PROG_PREFIX").unwrap_or_default() + utility;

    clap_complete::generate(shell, &mut command, bin_name, &mut io::stdout());
    io::stdout().flush().unwrap();
    process::exit(0);
}

/// # Errors
/// Returns an error if the writer fails.
#[allow(clippy::too_many_lines)]
fn main() -> io::Result<()> {
    let args: Vec<OsString> = uucore::args_os_filtered().collect();

    let mut tldr_zip = File::open("docs/tldr.zip")
        .ok()
        .and_then(|f| ZipArchive::new(f).ok());

    // Check for manpage/completion commands first
    if args.len() > 1 {
        let command = args.get(1).and_then(|s| s.to_str()).unwrap_or_default();
        match command {
            "manpage" => {
                let args_iter = args.into_iter().skip(2);
                gen_manpage(
                    &mut tldr_zip,
                    args_iter,
                    &util_map::<Box<dyn Iterator<Item = OsString>>>(),
                );
            }
            "completion" => {
                let args_iter = args.into_iter().skip(2);
                gen_completions(args_iter, &util_map::<Box<dyn Iterator<Item = OsString>>>());
            }
            "--help" | "-h" => {
                usage(&util_map::<Box<dyn Iterator<Item = OsString>>>());
                process::exit(0);
            }
            _ => {
                eprintln!("Unknown command: {command}");
                eprintln!("Use 'uudoc --help' for usage information.");
                process::exit(1);
            }
        }
    }
    let utils = util_map::<Box<dyn Iterator<Item = OsString>>>();
    // Initialize localization for uucore common strings (used by tldr example attribution)
    let _ = uucore::locale::setup_localization("uudoc");
    match std::fs::create_dir("docs/src/utils/") {
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        x => x,
    }?;

    println!("Writing initial info to SUMMARY.md");
    let mut summary = File::create("docs/src/SUMMARY.md")?;

    let _ = write!(
        summary,
        "# Summary\n\
        \n\
        [Introduction](index.md)\n\
        * [Installation](installation.md)\n\
        * [Build from source](build.md)\n\
        * [Platform support](platforms.md)\n\
        * [Contributing](CONTRIBUTING.md)\n\
        \t* [Development](DEVELOPMENT.md)\n\
        \t* [Code of Conduct](CODE_OF_CONDUCT.md)\n\
        * [GNU test coverage](test_coverage.md)\n\
        * [Extensions](extensions.md)\n\
        \n\
        # Reference\n\
        * [Multi-call binary](multicall.md)\n",
    );

    println!("Gathering utils per platform");
    let utils_per_platform = {
        let mut map = HashMap::new();
        for platform in ["unix", "macos", "windows", "unix_android"] {
            let platform_utils: Vec<String> = String::from_utf8(
                process::Command::new("./util/show-utils.sh")
                    .arg(format!("--features=feat_os_{platform}"))
                    .output()?
                    .stdout,
            )
            .unwrap()
            .trim()
            .split(' ')
            .map(ToString::to_string)
            .collect();
            map.insert(platform, platform_utils);
        }

        // Linux is a special case because it can support selinux
        let platform_utils: Vec<String> = String::from_utf8(
            process::Command::new("./util/show-utils.sh")
                .arg("--features=feat_os_unix feat_selinux")
                .output()?
                .stdout,
        )
        .unwrap()
        .trim()
        .split(' ')
        .map(ToString::to_string)
        .collect();
        map.insert("linux", platform_utils);

        map
    };

    let mut utils = utils.entries().collect::<Vec<_>>();
    utils.sort();

    println!("Writing util per platform table");
    {
        let mut platform_table_file = File::create("docs/src/platform_table.md").unwrap();

        // sum, cksum, b2sum, etc. are all available on all platforms, but not in the data structure
        // otherwise, we check the map for the util name.
        let check_supported = |name: &str, platform: &str| {
            if name.ends_with("sum") || utils_per_platform[platform].iter().any(|u| u == name) {
                "✓"
            } else {
                " "
            }
        };
        writeln!(
            platform_table_file,
            "| util             | Linux | macOS | Windows | FreeBSD | Android |\n\
             | ---------------- | ----- | ----- | ------- | ------- | ------- |"
        )?;
        for &(&name, _) in &utils {
            if name == "[" {
                continue;
            }
            // The alignment is not necessary, but makes the output a bit more
            // pretty when viewed as plain markdown.
            writeln!(
                platform_table_file,
                "| {:<16} | {:<5} | {:<5} | {:<7} | {:<7} | {:<7} |",
                format!("**{name}**"),
                check_supported(name, "linux"),
                check_supported(name, "macos"),
                check_supported(name, "windows"),
                check_supported(name, "unix"),
                check_supported(name, "unix_android"),
            )?;
        }
    }

    println!("Writing to utils");
    for (&name, (_, command)) in utils {
        let (utils_name, usage_name, command) = match name {
            "[" => {
                continue;
            }
            n => (n, n, command),
        };
        let p = format!("docs/src/utils/{usage_name}.md");

        let fluent = File::open(format!("src/uu/{utils_name}/locales/en-US.ftl"))
            .and_then(|mut f: File| {
                let mut s = String::new();
                f.read_to_string(&mut s)?;
                Ok(s)
            })
            .ok();

        if let Ok(f) = File::create(&p) {
            MDWriter {
                w: Box::new(f),
                command: command(),
                name: usage_name,
                tldr_zip: &mut tldr_zip,
                utils_per_platform: &utils_per_platform,
                fluent,
                fluent_key: utils_name.to_string(),
            }
            .markdown()?;
            println!("Wrote to '{p}'");
        } else {
            println!("Error writing to {p}");
        }
        writeln!(summary, "* [{usage_name}](utils/{usage_name}.md)")?;
    }
    Ok(())
}

fn fix_usage(name: &str, usage: String) -> String {
    match name {
        "test" => {
            // replace to [ but not the first two line
            usage
                .lines()
                .enumerate()
                .map(|(i, l)| {
                    if i > 1 {
                        l.replace("test", "[")
                    } else {
                        l.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => usage,
    }
}

struct MDWriter<'a, 'b> {
    w: Box<dyn Write>,
    command: Command,
    name: &'a str,
    tldr_zip: &'b mut Option<ZipArchive<File>>,
    utils_per_platform: &'b HashMap<&'b str, Vec<String>>,
    fluent: Option<String>,
    fluent_key: String,
}

impl MDWriter<'_, '_> {
    /// # Errors
    /// Returns an error if the writer fails.
    fn markdown(&mut self) -> io::Result<()> {
        write!(self.w, "# {}\n\n", self.name)?;
        self.additional()?;
        self.usage()?;
        self.about()?;
        self.options()?;
        self.after_help()?;
        self.examples()
    }

    /// Extract value for a Fluent key from the .ftl content
    fn extract_fluent_value(&self, key: &str) -> Option<String> {
        let content = self.fluent.as_ref()?;
        let resource = parser::parse(content.clone()).ok()?;

        for entry in resource.body {
            if let Entry::Message(Message {
                id,
                value: Some(Pattern { elements }),
                ..
            }) = entry
            {
                if id.name == key {
                    // Simple text extraction - just concatenate text elements
                    let mut result = String::new();
                    use fluent_syntax::ast::{
                        Expression, InlineExpression,
                        PatternElement::{Placeable, TextElement},
                    };
                    for element in elements {
                        if let TextElement { ref value } = element {
                            result.push_str(value);
                        }
                        if let Placeable {
                            expression:
                                Expression::Inline(InlineExpression::StringLiteral { ref value }),
                        } = element
                        {
                            result.push_str(value);
                        }
                    }
                    return Some(result);
                }
            }
        }
        None
    }

    /// # Errors
    /// Returns an error if the writer fails.
    fn additional(&mut self) -> io::Result<()> {
        writeln!(self.w, "<div class=\"additional\">")?;
        self.platforms()?;
        self.version()?;
        writeln!(self.w, "</div>")
    }

    /// # Errors
    /// Returns an error if the writer fails.
    fn platforms(&mut self) -> io::Result<()> {
        writeln!(self.w, "<div class=\"platforms\">")?;
        for (feature, icon) in [
            ("linux", "linux"),
            // freebsd is disabled for now because mdbook does not use font-awesome 5 yet.
            // ("unix", "freebsd"),
            ("macos", "apple"),
            ("windows", "windows"),
        ] {
            if self.name.contains("sum")
                || self.utils_per_platform[feature]
                    .iter()
                    .any(|u| u == self.name)
            {
                writeln!(self.w, "<i class=\"fa fa-brands fa-{icon}\"></i>")?;
            }
        }
        writeln!(self.w, "</div>")?;

        Ok(())
    }

    /// # Errors
    /// Returns an error if the writer fails.
    /// # Panics
    /// Panics if the version is not found.
    fn version(&mut self) -> io::Result<()> {
        writeln!(
            self.w,
            "<div class=\"version\">v{}</div>",
            self.command.render_version().split_once(' ').unwrap().1
        )
    }

    /// # Errors
    /// Returns an error if the writer fails.
    fn usage(&mut self) -> io::Result<()> {
        if let Some(usage) = self.extract_fluent_value(&format!("{}-usage", self.fluent_key)) {
            let usage = fix_usage(self.name, usage);
            writeln!(self.w, "\n```")?;
            writeln!(self.w, "{usage}")?;
            writeln!(self.w, "```")
        } else {
            Ok(())
        }
    }

    /// # Errors
    /// Returns an error if the writer fails.
    fn about(&mut self) -> io::Result<()> {
        if let Some(about) = self.extract_fluent_value(&format!("{}-about", self.fluent_key)) {
            writeln!(self.w, "{about}")
        } else {
            Ok(())
        }
    }

    /// # Errors
    /// Returns an error if the writer fails.
    fn after_help(&mut self) -> io::Result<()> {
        if let Some(after_help) =
            self.extract_fluent_value(&format!("{}-after-help", self.fluent_key))
        {
            writeln!(self.w, "\n\n{after_help}")
        } else {
            Ok(())
        }
    }

    /// # Errors
    /// Returns an error if the writer fails.
    fn examples(&mut self) -> io::Result<()> {
        if let Some(zip) = self.tldr_zip {
            if let Ok(examples) = write_zip_examples(zip, self.name, true) {
                writeln!(self.w, "{examples}")?;
            }
        }
        Ok(())
    }

    /// # Errors
    /// Returns an error if the writer fails.
    fn options(&mut self) -> io::Result<()> {
        writeln!(self.w)?;
        writeln!(self.w, "## Options")?;
        writeln!(self.w)?;
        write!(self.w, "<dl>")?;
        for arg in self.command.get_arguments() {
            write!(self.w, "<dt>")?;
            let mut first = true;
            for l in arg.get_long_and_visible_aliases().unwrap_or_default() {
                if first {
                    first = false;
                } else {
                    write!(self.w, ", ")?;
                }
                write!(self.w, "<code>")?;
                write!(self.w, "--{l}")?;
                if let Some(names) = arg.get_value_names() {
                    write!(
                        self.w,
                        "={}",
                        names
                            .iter()
                            .map(|x| format!("&lt;{x}&gt;"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    )?;
                }
                write!(self.w, "</code>")?;
            }
            for s in arg.get_short_and_visible_aliases().unwrap_or_default() {
                if first {
                    first = false;
                } else {
                    write!(self.w, ", ")?;
                }
                write!(self.w, "<code>")?;
                write!(self.w, "-{s}")?;
                if let Some(names) = arg.get_value_names() {
                    write!(
                        self.w,
                        " {}",
                        names
                            .iter()
                            .map(|x| format!("&lt;{x}&gt;"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    )?;
                }
                write!(self.w, "</code>")?;
            }
            writeln!(self.w, "</dt>")?;
            let help_text = arg.get_help().unwrap_or_default().to_string();
            // Try to resolve Fluent key from the FTL file, otherwise use help text as-is.
            // We always attempt resolution because shared keys (e.g. "base-common-help-*")
            // don't necessarily start with the utility-specific prefix.
            let resolved_help = self.extract_fluent_value(&help_text).unwrap_or(help_text);
            writeln!(
                self.w,
                "<dd>\n\n{}\n\n</dd>",
                resolved_help.replace('\n', "<br />")
            )?;
        }
        writeln!(self.w, "</dl>\n")
    }
}

/// # Panics
/// Panics if the archive is not ok
fn get_zip_content(archive: &mut ZipArchive<impl Read + Seek>, name: &str) -> Option<String> {
    let mut s = String::new();
    archive.by_name(name).ok()?.read_to_string(&mut s).unwrap();
    Some(s)
}

/// Extract examples for tldr.zip. The file docs/tldr.zip must exists
///
/// ```sh
/// curl https://tldr.sh/assets/tldr.zip -o docs/tldr.zip
/// ```
///
/// # Errors
///
/// Returns an error if the tldr.zip file cannot be opened or read
fn write_zip_examples(
    archive: &mut ZipArchive<impl Read + Seek>,
    name: &str,
    output_markdown: bool,
) -> io::Result<String> {
    let content = if let Some(f) = get_zip_content(archive, &format!("pages/common/{name}.md")) {
        f
    } else if let Some(f) = get_zip_content(archive, &format!("pages/linux/{name}.md")) {
        f
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Could not find tldr examples for {name}"),
        ));
    };

    match format_examples(content, output_markdown) {
        Err(e) => Err(io::Error::other(format!(
            "Failed to format the tldr examples of {name}: {e}"
        ))),
        Ok(s) => Ok(s),
    }
}

/// Format examples using std::fmt::Write
fn format_examples(content: String, output_markdown: bool) -> Result<String, std::fmt::Error> {
    use std::fmt::Write;
    let mut s = String::new();
    writeln!(s)?;
    writeln!(s, "## Examples")?;
    writeln!(s)?;
    for line in content.lines().skip_while(|l| !l.starts_with('-')) {
        if let Some(l) = line.strip_prefix("- ") {
            writeln!(s, "{l}")?;
        } else if line.starts_with('`') {
            if output_markdown {
                writeln!(s, "```shell\n{}\n```", line.trim_matches('`'))?;
            } else {
                writeln!(s, "{}", line.trim_matches('`'))?;
            }
        } else if line.is_empty() {
            writeln!(s)?;
        } else {
            // println!("Not sure what to do with this line:");
            // println!("{line}");
        }
    }
    writeln!(s)?;
    writeln!(s, "> {}", get_message("uudoc-tldr-attribution"))?;
    writeln!(s, ">")?;
    writeln!(s, "> {}", get_message("uudoc-tldr-disclaimer"))?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_process_manpage_fixes_th_header() {
        // Test that command names are uppercased and date is removed
        let input =
            ".TH cat 1 \"cat (uutils coreutils) 0.7.0\"\n.SH NAME\ncat - concatenate files\n";
        let expected = ".TH CAT 1 \"2024-01-01\"\n.SH NAME\ncat - concatenate files\n";

        let result = post_process_manpage(input.to_string(), "2024-01-01");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_post_process_manpage_removes_trailing_whitespace() {
        // Test that trailing whitespace is removed from lines
        let input = ".TH TEST 1  \nSome text with trailing spaces   \n.SH SECTION  \n";
        let expected = ".TH TEST 1 \"2024-01-01\"\nSome text with trailing spaces\n.SH SECTION\n";

        let result = post_process_manpage(input.to_string(), "2024-01-01");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_post_process_manpage_fixes_double_br() {
        // Test that redundant .br macros are removed
        let input = ".TH TEST 1\n.br\n.br\nSome text\n";
        let expected = ".TH TEST 1 \"2024-01-01\"\n.br\nSome text\n";

        let result = post_process_manpage(input.to_string(), "2024-01-01");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_post_process_manpage_fixes_br_with_empty_line() {
        // Test that .br with empty line patterns are fixed
        // Both .br macros should be removed (first because followed by empty, second because preceded by empty)
        let input = ".TH TEST 1\n.br\n\n.br\nSome text\n";
        let expected = ".TH TEST 1 \"2024-01-01\"\n\nSome text\n";

        let result = post_process_manpage(input.to_string(), "2024-01-01");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_post_process_manpage_preserves_single_br() {
        // Test that single .br macros are preserved
        let input = ".TH TEST 1\nLine 1\n.br\nLine 2\n";
        let expected = ".TH TEST 1 \"2024-01-01\"\nLine 1\n.br\nLine 2\n";

        let result = post_process_manpage(input.to_string(), "2024-01-01");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_post_process_manpage_handles_mixed_case_command() {
        // Test that mixed case command names are uppercased
        let input = ".TH CaT 1 \"some version info\"\nContent\n";
        let expected = ".TH CAT 1 \"2024-01-01\"\nContent\n";

        let result = post_process_manpage(input.to_string(), "2024-01-01");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_post_process_manpage_handles_no_th_header() {
        // Test that manpages without TH headers are handled gracefully
        let input = ".SH NAME\ntest - a test utility\n";
        let expected = ".SH NAME\ntest - a test utility\n";

        let result = post_process_manpage(input.to_string(), "2024-01-01");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_post_process_manpage_complex_br_pattern() {
        // Test complex .br patterns with multiple occurrences
        let input =
            ".TH TEST 1\nSection 1\n.br\n\n.br\nMiddle\n.br\n.br\nSection 2\n.br\n\n.br\nEnd\n";
        // .br followed/preceded by empty lines should be removed, consecutive .br should have one removed
        let expected = ".TH TEST 1 \"2024-01-01\"\nSection 1\n\nMiddle\n.br\nSection 2\n\nEnd\n";

        let result = post_process_manpage(input.to_string(), "2024-01-01");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_post_process_manpage_malformed_th_header() {
        // Test that malformed TH headers don't cause panics and are handled gracefully
        let input1 = ".TH\nContent\n"; // Missing command name
        let expected1 = ".TH\nContent\n";
        let result1 = post_process_manpage(input1.to_string(), "2024-01-01");
        assert_eq!(result1, expected1);

        // TH header with special characters
        let input2 = ".TH test-cmd 1 \"version 1.0\"\nContent\n";
        let expected2 = ".TH TEST-CMD 1 \"2024-01-01\"\nContent\n";
        let result2 = post_process_manpage(input2.to_string(), "2024-01-01");
        assert_eq!(result2, expected2);

        // TH header at end of file without newline
        let input3 = "Content\n.TH test 1";
        let expected3 = "Content\n.TH TEST 1 \"2024-01-01\"\n";
        let result3 = post_process_manpage(input3.to_string(), "2024-01-01");
        assert_eq!(result3, expected3);

        // Multiple TH headers (only first should be processed due to ^anchor)
        let input4 = ".TH first 1\nMiddle\n.TH second 1\n";
        let expected4 = ".TH FIRST 1 \"2024-01-01\"\nMiddle\n.TH SECOND 1 \"2024-01-01\"\n";
        let result4 = post_process_manpage(input4.to_string(), "2024-01-01");
        assert_eq!(result4, expected4);
    }

    #[test]
    fn test_post_process_manpage_removes_br_before_empty_line() {
        // Test that .br is removed when followed by empty line (which becomes .sp)
        let input = ".TH TEST 1\nSome text\n.br\n\nMore text\n";
        let expected = ".TH TEST 1 \"2024-01-01\"\nSome text\n\nMore text\n";

        let result = post_process_manpage(input.to_string(), "2024-01-01");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_post_process_manpage_complex_br_before_empty() {
        // Test multiple .br before empty line patterns
        let input = ".TH TEST 1\nSection 1\n.br\n\nSection 2\n.br\n\nSection 3\n";
        let expected = ".TH TEST 1 \"2024-01-01\"\nSection 1\n\nSection 2\n\nSection 3\n";

        let result = post_process_manpage(input.to_string(), "2024-01-01");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_post_process_manpage_removes_br_after_empty_line() {
        // Test that .br is removed when preceded by empty line (which becomes .sp)
        let input = ".TH TEST 1\nSome text\n\n.br\nMore text\n";
        let expected = ".TH TEST 1 \"2024-01-01\"\nSome text\n\nMore text\n";

        let result = post_process_manpage(input.to_string(), "2024-01-01");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_post_process_manpage_fixes_escape_sequences() {
        // Test that \\\\0 and \\0 are fixed to \e0 (literal backslash-zero)
        let input = ".TH TEST 1\nText with \\\\\\\\0 and \\\\0 escape\n";
        let expected = ".TH TEST 1 \"2024-01-01\"\nText with \\e0 and \\e0 escape\n";

        let result = post_process_manpage(input.to_string(), "2024-01-01");
        assert_eq!(result, expected);
    }

    /// Helper to create an MDWriter with given FTL content for testing
    fn make_test_writer(fluent_content: &str, fluent_key: &str) -> MDWriter<'static, 'static> {
        // Leak the HashMap to get a 'static reference (fine in tests)
        let platforms: &'static HashMap<&'static str, Vec<String>> =
            Box::leak(Box::new(HashMap::new()));
        let tldr_zip: &'static mut Option<ZipArchive<File>> = Box::leak(Box::new(None));
        MDWriter {
            w: Box::new(Vec::new()),
            command: Command::new("test"),
            name: "test",
            tldr_zip,
            utils_per_platform: platforms,
            fluent: Some(fluent_content.to_string()),
            fluent_key: fluent_key.to_string(),
        }
    }

    #[test]
    fn test_extract_fluent_value_resolves_utility_specific_keys() {
        let ftl = "base32-about = encode/decode data and print to standard output\n\
                    base32-usage = base32 [OPTION]... [FILE]\n";
        let writer = make_test_writer(ftl, "base32");

        assert_eq!(
            writer.extract_fluent_value("base32-about"),
            Some("encode/decode data and print to standard output".to_string())
        );
        assert_eq!(
            writer.extract_fluent_value("base32-usage"),
            Some("base32 [OPTION]... [FILE]".to_string())
        );
    }

    #[test]
    fn test_extract_fluent_value_resolves_shared_keys() {
        // Regression test: shared Fluent keys like "base-common-help-decode"
        // don't start with the utility prefix "base32-". They must still be
        // resolved from the same FTL file.
        let ftl = "base32-about = encode/decode data\n\
                    base-common-help-decode = decode data\n\
                    base-common-help-ignore-garbage = when decoding, ignore non-alphabet characters\n";
        let writer = make_test_writer(ftl, "base32");

        assert_eq!(
            writer.extract_fluent_value("base-common-help-decode"),
            Some("decode data".to_string())
        );
        assert_eq!(
            writer.extract_fluent_value("base-common-help-ignore-garbage"),
            Some("when decoding, ignore non-alphabet characters".to_string())
        );
    }

    #[test]
    fn test_extract_fluent_value_returns_none_for_missing_keys() {
        let ftl = "base32-about = encode/decode data\n";
        let writer = make_test_writer(ftl, "base32");

        assert_eq!(writer.extract_fluent_value("nonexistent-key"), None);
    }

    #[test]
    fn test_extract_fluent_value_returns_none_when_no_ftl() {
        let platforms: &'static HashMap<&'static str, Vec<String>> =
            Box::leak(Box::new(HashMap::new()));
        let tldr_zip: &'static mut Option<ZipArchive<File>> = Box::leak(Box::new(None));
        let writer = MDWriter {
            w: Box::new(Vec::new()),
            command: Command::new("test"),
            name: "test",
            tldr_zip,
            utils_per_platform: platforms,
            fluent: None,
            fluent_key: "test".to_string(),
        };

        assert_eq!(writer.extract_fluent_value("any-key"), None);
    }

    #[test]
    fn test_options_resolves_shared_fluent_keys_in_help_text() {
        // End-to-end test: an option whose help text is a shared Fluent key
        // must have that key resolved in the generated markdown output.
        let ftl = "base-common-help-decode = decode data\n";
        let command = Command::new("base32").arg(
            Arg::new("decode")
                .short('d')
                .long("decode")
                .help("base-common-help-decode")
                .action(clap::ArgAction::SetTrue),
        );
        let platforms: &'static HashMap<&'static str, Vec<String>> =
            Box::leak(Box::new(HashMap::new()));
        let tldr_zip: &'static mut Option<ZipArchive<File>> = Box::leak(Box::new(None));
        let mut writer = MDWriter {
            w: Box::new(Vec::<u8>::new()),
            command,
            name: "base32",
            tldr_zip,
            utils_per_platform: platforms,
            fluent: Some(ftl.to_string()),
            fluent_key: "base32".to_string(),
        };

        writer.options().unwrap();

        // Recover the output buffer
        let output = {
            let buf = writer.w.as_ref() as *const dyn Write as *const Vec<u8>;
            // SAFETY: we know the writer wraps a Vec<u8>
            unsafe { &*buf }
        };
        let html = String::from_utf8_lossy(output);

        // The resolved text "decode data" must appear, NOT the raw key
        assert!(
            html.contains("decode data"),
            "Expected resolved help text 'decode data', got:\n{html}"
        );
        assert!(
            !html.contains("base-common-help-decode"),
            "Raw Fluent key should not appear in output, got:\n{html}"
        );
    }
}
