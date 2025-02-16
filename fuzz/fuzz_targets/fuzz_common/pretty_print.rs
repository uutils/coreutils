// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::fmt;

use console::{style, Style};
use similar::TextDiff;

pub fn print_section<S: fmt::Display>(s: S) {
    println!("{}", style(format!("=== {}", s)).bold());
}

pub fn print_subsection<S: fmt::Display>(s: S) {
    println!("{}", style(format!("--- {}", s)).bright());
}

pub fn print_test_begin<S: fmt::Display>(msg: S) {
    println!(
        "{} {} {}",
        style("===").bold(), // Kind of gray
        style("TEST").black().on_yellow().bold(),
        style(msg).bold()
    );
}

pub fn print_end_with_status<S: fmt::Display>(msg: S, ok: bool) {
    let ok = if ok {
        style(" OK ").black().on_green().bold()
    } else {
        style(" KO ").black().on_red().bold()
    };

    println!(
        "{} {} {}",
        style("===").bold(), // Kind of gray
        ok,
        style(msg).bold()
    );
}

pub fn print_or_empty(s: &str) {
    let to_print = if s.is_empty() { "(empty)" } else { s };

    println!("{}", style(to_print).dim());
}

pub fn print_with_style<S: fmt::Display>(msg: S, style: Style) {
    println!("{}", style.apply_to(msg));
}

pub fn print_diff<'a, 'b>(got: &'a str, expected: &'b str) {
    let diff = TextDiff::from_lines(got, expected);

    print_subsection("START diff");

    for change in diff.iter_all_changes() {
        let (sign, style) = match change.tag() {
            similar::ChangeTag::Equal => (" ", Style::new().dim()),
            similar::ChangeTag::Delete => ("-", Style::new().red()),
            similar::ChangeTag::Insert => ("+", Style::new().green()),
        };
        print!("{}{}", style.apply_to(sign).bold(), style.apply_to(change));
    }

    print_subsection("END diff");
    println!();
}
