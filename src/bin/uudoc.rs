// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore tldr

use clap::Command;
use std::ffi::OsString;
use std::fs::File;
use std::io::Cursor;
use std::io::{self, Read, Seek, Write};
use zip::ZipArchive;

include!(concat!(env!("OUT_DIR"), "/uutils_map.rs"));

fn main() -> io::Result<()> {
    println!("Downloading tldr archive");
    let mut zip_reader = ureq::get("https://tldr.sh/assets/tldr.zip")
        .call()
        .unwrap()
        .into_reader();
    let mut buffer = Vec::new();
    zip_reader.read_to_end(&mut buffer).unwrap();
    let mut tldr_zip = ZipArchive::new(Cursor::new(buffer)).unwrap();

    let utils = util_map::<Box<dyn Iterator<Item = OsString>>>();
    match std::fs::create_dir("docs/src/utils/") {
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        x => x,
    }?;

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

    let mut utils = utils.entries().collect::<Vec<_>>();
    utils.sort();
    for (&name, (_, command)) in utils {
        if name == "[" {
            continue;
        }
        let p = format!("docs/src/utils/{}.md", name);
        if let Ok(f) = File::create(&p) {
            write_markdown(f, &mut command(), name, &mut tldr_zip)?;
            println!("Wrote to '{}'", p);
        } else {
            println!("Error writing to {}", p);
        }
        writeln!(summary, "* [{0}](utils/{0}.md)", name)?;
    }
    Ok(())
}

fn write_markdown(
    mut w: impl Write,
    command: &mut Command,
    name: &str,
    tldr_zip: &mut zip::ZipArchive<impl Read + Seek>,
) -> io::Result<()> {
    write!(w, "# {}\n\n", name)?;
    write_version(&mut w, command)?;
    write_usage(&mut w, command, name)?;
    write_description(&mut w, command)?;
    write_options(&mut w, command)?;
    write_examples(&mut w, name, tldr_zip)
}

fn write_version(w: &mut impl Write, command: &Command) -> io::Result<()> {
    writeln!(
        w,
        "<div class=\"version\">version: {}</div>",
        command.render_version().split_once(' ').unwrap().1
    )
}

fn write_usage(w: &mut impl Write, command: &mut Command, name: &str) -> io::Result<()> {
    writeln!(w, "\n```")?;
    let mut usage: String = command
        .render_usage()
        .lines()
        .skip(1)
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    usage = usage.replace(uucore::execution_phrase(), name);
    writeln!(w, "{}", usage)?;
    writeln!(w, "```")
}

fn write_description(w: &mut impl Write, command: &Command) -> io::Result<()> {
    if let Some(about) = command.get_long_about().or_else(|| command.get_about()) {
        writeln!(w, "{}", about)
    } else {
        Ok(())
    }
}

fn write_examples(
    w: &mut impl Write,
    name: &str,
    tldr_zip: &mut zip::ZipArchive<impl Read + Seek>,
) -> io::Result<()> {
    let content = if let Some(f) = get_zip_content(tldr_zip, &format!("pages/common/{}.md", name)) {
        f
    } else if let Some(f) = get_zip_content(tldr_zip, &format!("pages/linux/{}.md", name)) {
        f
    } else {
        return Ok(());
    };

    writeln!(w, "## Examples")?;
    writeln!(w)?;
    for line in content.lines().skip_while(|l| !l.starts_with('-')) {
        if let Some(l) = line.strip_prefix("- ") {
            writeln!(w, "{}", l)?;
        } else if line.starts_with('`') {
            writeln!(w, "```shell\n{}\n```", line.trim_matches('`'))?;
        } else if line.is_empty() {
            writeln!(w)?;
        } else {
            println!("Not sure what to do with this line:");
            println!("{}", line);
        }
    }
    writeln!(w)?;
    writeln!(
        w,
        "> The examples are provided by the [tldr-pages project](https://tldr.sh) under the [CC BY 4.0 License](https://github.com/tldr-pages/tldr/blob/main/LICENSE.md)."
    )?;
    writeln!(w, ">")?;
    writeln!(
        w,
        "> Please note that, as uutils is a work in progress, some examples might fail."
    )
}

fn get_zip_content(archive: &mut ZipArchive<impl Read + Seek>, name: &str) -> Option<String> {
    let mut s = String::new();
    archive.by_name(name).ok()?.read_to_string(&mut s).unwrap();
    Some(s)
}

fn write_options(w: &mut impl Write, command: &Command) -> io::Result<()> {
    writeln!(w, "<h2>Options</h2>")?;
    write!(w, "<dl>")?;
    for arg in command.get_arguments() {
        write!(w, "<dt>")?;
        let mut first = true;
        for l in arg.get_long_and_visible_aliases().unwrap_or_default() {
            if !first {
                write!(w, ", ")?;
            } else {
                first = false;
            }
            write!(w, "<code>")?;
            write!(w, "--{}", l)?;
            if let Some(names) = arg.get_value_names() {
                write!(
                    w,
                    "={}",
                    names
                        .iter()
                        .map(|x| format!("&lt;{}&gt;", x))
                        .collect::<Vec<_>>()
                        .join(" ")
                )?;
            }
            write!(w, "</code>")?;
        }
        for s in arg.get_short_and_visible_aliases().unwrap_or_default() {
            if !first {
                write!(w, ", ")?;
            } else {
                first = false;
            }
            write!(w, "<code>")?;
            write!(w, "-{}", s)?;
            if let Some(names) = arg.get_value_names() {
                write!(
                    w,
                    " {}",
                    names
                        .iter()
                        .map(|x| format!("&lt;{}&gt;", x))
                        .collect::<Vec<_>>()
                        .join(" ")
                )?;
            }
            write!(w, "</code>")?;
        }
        writeln!(w, "</dt>")?;
        writeln!(
            w,
            "<dd>\n\n{}\n\n</dd>",
            arg.get_help().unwrap_or_default().replace('\n', "<br />")
        )?;
    }
    writeln!(w, "</dl>\n")
}
