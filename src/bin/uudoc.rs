// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::App;
use std::collections::hash_map::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Write};

include!(concat!(env!("OUT_DIR"), "/uutils_map.rs"));

fn main() -> io::Result<()> {
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
        \n\
        # Reference\n\
        * [Multi-call binary](multicall.md)\n",
    );

    let mut utils = utils.iter().collect::<Vec<_>>();
    utils.sort();
    for (&name, (_, app)) in utils {
        if name == "[" {
            continue;
        }
        let p = format!("docs/src/utils/{}.md", name);
        if let Ok(f) = File::create(&p) {
            write_markdown(f, &mut app(), name)?;
            println!("Wrote to '{}'", p);
        } else {
            println!("Error writing to {}", p);
        }
        writeln!(summary, "* [{0}](utils/{0}.md)", name)?
    }
    Ok(())
}

fn write_markdown(mut w: impl Write, app: &mut App, name: &str) -> io::Result<()> {
    write!(w, "# {}\n\n", name)?;
    write_version(&mut w, app)?;
    write_usage(&mut w, app, name)?;
    write_description(&mut w, app)?;
    write_options(&mut w, app)
}

fn write_version(w: &mut impl Write, app: &App) -> io::Result<()> {
    writeln!(
        w,
        "<div class=\"version\">version: {}</div>",
        app.render_version().split_once(' ').unwrap().1
    )
}

fn write_usage(w: &mut impl Write, app: &mut App, name: &str) -> io::Result<()> {
    writeln!(w, "\n```")?;
    let mut usage: String = app.render_usage().lines().nth(1).unwrap().trim().into();
    usage = usage.replace(app.get_name(), name);
    writeln!(w, "{}", usage)?;
    writeln!(w, "```")
}

fn write_description(w: &mut impl Write, app: &App) -> io::Result<()> {
    if let Some(about) = app.get_long_about().or_else(|| app.get_about()) {
        writeln!(w, "{}", about)
    } else {
        Ok(())
    }
}

fn write_options(w: &mut impl Write, app: &App) -> io::Result<()> {
    writeln!(w, "<h2>Options</h2>")?;
    write!(w, "<dl>")?;
    for arg in app.get_arguments() {
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
        writeln!(w, "<dd>\n\n{}\n\n</dd>", arg.get_help().unwrap_or_default())?;
    }
    writeln!(w, "</dl>")
}
