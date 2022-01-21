// This file is part of the uutils coreutils package.
//
// (c) Michael Gehring <mg@ebfe.org>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::App;
use std::collections::hash_map::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::Write;

include!(concat!(env!("OUT_DIR"), "/uutils_map.rs"));

fn main() -> std::io::Result<()> {
    let utils = util_map::<Box<dyn Iterator<Item = OsString>>>();
    match std::fs::create_dir("docs/src/utils/") {
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        x => x,
    }?;
    for (name, (_, app)) in utils {
        let p = format!("docs/src/utils/{}.md", name);
        if let Ok(f) = File::create(&p) {
            write_markdown(f, &mut app(), name);
            println!("Wrote to '{}'", p);
        } else {
            println!("Error writing to {}", p);
        }
    }
    Ok(())
}

fn write_markdown(mut w: impl Write, app: &mut App, name: &str) {
    let _ = write!(w, "# {}\n\n", name);
    write_version(&mut w, app);
    write_usage(&mut w, app, name);
    write_summary(&mut w, app);
    write_options(&mut w, app);
}

fn write_version(w: &mut impl Write, app: &App) {
    let _ = writeln!(
        w,
        "<div class=\"version\">version: {}</div>",
        app.render_version().split_once(' ').unwrap().1
    );
}

fn write_usage(w: &mut impl Write, app: &mut App, name: &str) {
    let _ = writeln!(w, "\n```");
    let mut usage: String = app.render_usage().lines().nth(1).unwrap().trim().into();
    usage = usage.replace(app.get_name(), name);
    let _ = writeln!(w, "{}", usage);
    let _ = writeln!(w, "```");
}

fn write_summary(w: &mut impl Write, app: &App) {
    if let Some(about) = app.get_long_about().or_else(|| app.get_about()) {
        let _ = writeln!(w, "{}", about);
    }
}

fn write_options(w: &mut impl Write, app: &App) {
    let _ = writeln!(w, "<h2>Options</h2>");
    let _ = write!(w, "<dl>");
    for arg in app.get_arguments() {
        let _ = write!(w, "<dt>");
        let mut first = true;
        for l in arg.get_long_and_visible_aliases().unwrap_or_default() {
            if !first {
                let _ = write!(w, ", ");
            } else {
                first = false;
            }
            let _ = write!(w, "<code>");
            let _ = write!(w, "--{}", l);
            if let Some(names) = arg.get_value_names() {
                let _ = write!(
                    w,
                    "={}",
                    names
                        .iter()
                        .map(|x| format!("&lt;{}&gt;", x))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            }
            let _ = write!(w, "</code>");
        }
        for s in arg.get_short_and_visible_aliases().unwrap_or_default() {
            if !first {
                let _ = write!(w, ", ");
            } else {
                first = false;
            }
            let _ = write!(w, "<code>");
            let _ = write!(w, "-{}", s);
            if let Some(names) = arg.get_value_names() {
                let _ = write!(
                    w,
                    " {}",
                    names
                        .iter()
                        .map(|x| format!("&lt;{}&gt;", x))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            }
            let _ = write!(w, "</code>");
        }
        let _ = writeln!(w, "</dt>");
        let _ = writeln!(w, "<dd>{}</dd>", arg.get_help().unwrap_or_default());
    }
    let _ = writeln!(w, "</dl>");
}
