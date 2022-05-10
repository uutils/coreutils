// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore tldr

use clap::Command;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Read, Seek, Write};
use zip::ZipArchive;

include!(concat!(env!("OUT_DIR"), "/uutils_map.rs"));

fn main() -> io::Result<()> {
    let mut tldr_zip = File::open("docs/tldr.zip")
        .ok()
        .and_then(|f| ZipArchive::new(f).ok());

    if tldr_zip.is_none() {
        println!("Warning: No tldr archive found, so the documentation will not include examples.");
        println!("To include examples in the documentation, download the tldr archive and put it in the docs/ folder.");
        println!();
        println!("  curl https://tldr.sh/assets/tldr.zip -o docs/tldr.zip");
        println!();
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
        * [Contributing](contributing.md)\n\
        * [GNU test coverage](test_coverage.md)\n\
        \n\
        # Reference\n\
        * [Multi-call binary](multicall.md)\n",
    );

    println!("Gathering utils per platform");
    let utils_per_platform = {
        let mut map = HashMap::new();
        for platform in ["unix", "macos", "windows"] {
            let platform_utils: Vec<String> = String::from_utf8(
                std::process::Command::new("./util/show-utils.sh")
                    .arg(format!("--features=feat_os_{}", platform))
                    .output()?
                    .stdout,
            )
            .unwrap()
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
        .split(' ')
        .map(ToString::to_string)
        .collect();
        map.insert("linux", platform_utils);

        map
    };

    println!("Writing to utils");
    let mut utils = utils.entries().collect::<Vec<_>>();
    utils.sort();
    for (&name, (_, command)) in utils {
        if name == "[" {
            continue;
        }
        let p = format!("docs/src/utils/{}.md", name);
        if let Ok(f) = File::create(&p) {
            MDWriter {
                w: Box::new(f),
                command: command(),
                name,
                tldr_zip: &mut tldr_zip,
                utils_per_platform: &utils_per_platform,
            }
            .markdown()?;
            println!("Wrote to '{}'", p);
        } else {
            println!("Error writing to {}", p);
        }
        writeln!(summary, "* [{0}](utils/{0}.md)", name)?;
    }
    Ok(())
}

struct MDWriter<'a, 'b> {
    w: Box<dyn Write>,
    command: Command<'a>,
    name: &'a str,
    tldr_zip: &'b mut Option<ZipArchive<File>>,
    utils_per_platform: &'b HashMap<&'b str, Vec<String>>,
}

impl<'a, 'b> MDWriter<'a, 'b> {
    fn markdown(&mut self) -> io::Result<()> {
        write!(self.w, "# {}\n\n", self.name)?;
        self.additional()?;
        self.usage()?;
        self.description()?;
        self.options()?;
        self.examples()
    }

    fn additional(&mut self) -> io::Result<()> {
        writeln!(self.w, "<div class=\"additional\">")?;
        self.platforms()?;
        self.version()?;
        writeln!(self.w, "</div>")
    }

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
                writeln!(self.w, "<i class=\"fa fa-brands fa-{}\"></i>", icon)?;
            }
        }
        writeln!(self.w, "</div>")?;

        Ok(())
    }

    fn version(&mut self) -> io::Result<()> {
        writeln!(
            self.w,
            "<div class=\"version\">v{}</div>",
            self.command.render_version().split_once(' ').unwrap().1
        )
    }

    fn usage(&mut self) -> io::Result<()> {
        writeln!(self.w, "\n```")?;
        let mut usage: String = self
            .command
            .render_usage()
            .lines()
            .skip(1)
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        usage = usage.replace(uucore::execution_phrase(), self.name);
        writeln!(self.w, "{}", usage)?;
        writeln!(self.w, "```")
    }

    fn description(&mut self) -> io::Result<()> {
        if let Some(about) = self
            .command
            .get_long_about()
            .or_else(|| self.command.get_about())
        {
            writeln!(self.w, "{}", about)
        } else {
            Ok(())
        }
    }

    fn examples(&mut self) -> io::Result<()> {
        if let Some(zip) = self.tldr_zip {
            let content = if let Some(f) =
                get_zip_content(zip, &format!("pages/common/{}.md", self.name))
            {
                f
            } else if let Some(f) = get_zip_content(zip, &format!("pages/linux/{}.md", self.name)) {
                f
            } else {
                return Ok(());
            };

            writeln!(self.w, "## Examples")?;
            writeln!(self.w)?;
            for line in content.lines().skip_while(|l| !l.starts_with('-')) {
                if let Some(l) = line.strip_prefix("- ") {
                    writeln!(self.w, "{}", l)?;
                } else if line.starts_with('`') {
                    writeln!(self.w, "```shell\n{}\n```", line.trim_matches('`'))?;
                } else if line.is_empty() {
                    writeln!(self.w)?;
                } else {
                    println!("Not sure what to do with this line:");
                    println!("{}", line);
                }
            }
            writeln!(self.w)?;
            writeln!(
                self.w,
                "> The examples are provided by the [tldr-pages project](https://tldr.sh) under the [CC BY 4.0 License](https://github.com/tldr-pages/tldr/blob/main/LICENSE.md)."
            )?;
            writeln!(self.w, ">")?;
            writeln!(
                self.w,
                "> Please note that, as uutils is a work in progress, some examples might fail."
            )?;
        }
        Ok(())
    }

    fn options(&mut self) -> io::Result<()> {
        writeln!(self.w, "<h2>Options</h2>")?;
        write!(self.w, "<dl>")?;
        for arg in self.command.get_arguments() {
            write!(self.w, "<dt>")?;
            let mut first = true;
            for l in arg.get_long_and_visible_aliases().unwrap_or_default() {
                if !first {
                    write!(self.w, ", ")?;
                } else {
                    first = false;
                }
                write!(self.w, "<code>")?;
                write!(self.w, "--{}", l)?;
                if let Some(names) = arg.get_value_names() {
                    write!(
                        self.w,
                        "={}",
                        names
                            .iter()
                            .map(|x| format!("&lt;{}&gt;", x))
                            .collect::<Vec<_>>()
                            .join(" ")
                    )?;
                }
                write!(self.w, "</code>")?;
            }
            for s in arg.get_short_and_visible_aliases().unwrap_or_default() {
                if !first {
                    write!(self.w, ", ")?;
                } else {
                    first = false;
                }
                write!(self.w, "<code>")?;
                write!(self.w, "-{}", s)?;
                if let Some(names) = arg.get_value_names() {
                    write!(
                        self.w,
                        " {}",
                        names
                            .iter()
                            .map(|x| format!("&lt;{}&gt;", x))
                            .collect::<Vec<_>>()
                            .join(" ")
                    )?;
                }
                write!(self.w, "</code>")?;
            }
            writeln!(self.w, "</dt>")?;
            writeln!(
                self.w,
                "<dd>\n\n{}\n\n</dd>",
                arg.get_help().unwrap_or_default().replace('\n', "<br />")
            )?;
        }
        writeln!(self.w, "</dl>\n")
    }
}

fn get_zip_content(archive: &mut ZipArchive<impl Read + Seek>, name: &str) -> Option<String> {
    let mut s = String::new();
    archive.by_name(name).ok()?.read_to_string(&mut s).unwrap();
    Some(s)
}
