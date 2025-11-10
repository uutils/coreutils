// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore mangen tldr

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
use textwrap::{fill, indent, termwidth};
use zip::ZipArchive;

use coreutils::validation;
use uucore::Args;

include!(concat!(env!("OUT_DIR"), "/uutils_map.rs"));

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
fn gen_coreutils_app<T: Args>(util_map: &UtilityMap<T>) -> clap::Command {
    let mut command = clap::Command::new("coreutils");
    for (name, (_, sub_app)) in util_map {
        // Recreate a small subcommand with only the relevant info
        // (name & short description)
        let about = sub_app()
            .get_about()
            .expect("Could not get the 'about'")
            .to_string();
        let sub_app = clap::Command::new(name).about(about);
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
        if let Some(zip) = tldr {
            if let Ok(examples) = write_zip_examples(zip, utility, false) {
                cmd = cmd.after_help(examples);
            }
        }
        cmd
    };

    let man = Man::new(command);
    man.render(&mut io::stdout())
        .expect("Man page generation failed");
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

/// print tldr error
fn print_tldr_error() {
    eprintln!("Warning: No tldr archive found, so the documentation will not include examples.");
    eprintln!(
        "To include examples in the documentation, download the tldr archive and put it in the docs/ folder."
    );
    eprintln!();
    eprintln!("  curl https://tldr.sh/assets/tldr.zip -o docs/tldr.zip");
    eprintln!();
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
                if tldr_zip.is_none() {
                    print_tldr_error();
                }
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
    if tldr_zip.is_none() {
        print_tldr_error();
    }
    let utils = util_map::<Box<dyn Iterator<Item = OsString>>>();
    match std::fs::create_dir("docs/src/utils/") {
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
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
                std::process::Command::new("./util/show-utils.sh")
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
            std::process::Command::new("./util/show-utils.sh")
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
        if name == "[" {
            continue;
        }
        let p = format!("docs/src/utils/{name}.md");

        let fluent = File::open(format!("src/uu/{name}/locales/en-US.ftl"))
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
                name,
                tldr_zip: &mut tldr_zip,
                utils_per_platform: &utils_per_platform,
                fluent,
            }
            .markdown()?;
            println!("Wrote to '{p}'");
        } else {
            println!("Error writing to {p}");
        }
        writeln!(summary, "* [{name}](utils/{name}.md)")?;
    }
    Ok(())
}

struct MDWriter<'a, 'b> {
    w: Box<dyn Write>,
    command: Command,
    name: &'a str,
    tldr_zip: &'b mut Option<ZipArchive<File>>,
    utils_per_platform: &'b HashMap<&'b str, Vec<String>>,
    fluent: Option<String>,
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
                    for element in elements {
                        if let fluent_syntax::ast::PatternElement::TextElement { value } = element {
                            result.push_str(&value);
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
        if let Some(usage) = self.extract_fluent_value(&format!("{}-usage", self.name)) {
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
        if let Some(about) = self.extract_fluent_value(&format!("{}-about", self.name)) {
            writeln!(self.w, "{about}")
        } else {
            Ok(())
        }
    }

    /// # Errors
    /// Returns an error if the writer fails.
    fn after_help(&mut self) -> io::Result<()> {
        if let Some(after_help) = self.extract_fluent_value(&format!("{}-after-help", self.name)) {
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
        writeln!(self.w, "<h2>Options</h2>")?;
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
            // Try to resolve Fluent key if it looks like one, otherwise use as-is
            let resolved_help = if help_text.starts_with(&format!("{}-help-", self.name)) {
                self.extract_fluent_value(&help_text).unwrap_or(help_text)
            } else {
                help_text
            };
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
        Err(e) => Err(std::io::Error::other(format!(
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
    writeln!(s, "Examples")?;
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
    writeln!(
        s,
        "> The examples are provided by the [tldr-pages project](https://tldr.sh) under the [CC BY 4.0 License](https://github.com/tldr-pages/tldr/blob/main/LICENSE.md)."
    )?;
    writeln!(s, ">")?;
    writeln!(
        s,
        "> Please note that, as uutils is a work in progress, some examples might fail."
    )?;
    Ok(s)
}
