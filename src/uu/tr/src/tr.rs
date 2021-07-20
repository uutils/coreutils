//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Michael Gehring <mg@ebfe.org>
//  * (c) kwantam <kwantam@gmail.com>
//  *     * 2015-04-28 ~ created `expand` module to eliminate most allocs during setup
//  * (c) Sergey "Shnatsel" Davidoff <shnatsel@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) allocs bset dflag cflag sflag tflag

#[macro_use]
extern crate uucore;
extern crate nom;

mod operation;
mod unicode_table;

use clap::{crate_version, App, Arg};
use nom::AsBytes;
use operation::{translate_input_new, Sequence, SqueezeOperation, TranslateOperation};
use std::io::{stdin, stdout, BufReader, BufWriter};

use crate::operation::DeleteOperation;
use uucore::InvalidEncodingHandling;

static ABOUT: &str = "translate or delete characters";

mod options {
    pub const COMPLEMENT: &str = "complement";
    pub const DELETE: &str = "delete";
    pub const SQUEEZE: &str = "squeeze-repeats";
    pub const TRUNCATE_SET1: &str = "truncate-set1";
    pub const SETS: &str = "sets";
}

fn get_usage() -> String {
    format!("{} [OPTION]... SET1 [SET2]", executable!())
}

fn get_long_usage() -> String {
    "Translate, squeeze, and/or delete characters from standard input,
writing to standard output."
        .to_string()
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let usage = get_usage();
    let after_help = get_long_usage();

    let matches = uu_app()
        .usage(&usage[..])
        .after_help(&after_help[..])
        .get_matches_from(args);

    let delete_flag = matches.is_present(options::DELETE);
    let complement_flag = matches.is_present(options::COMPLEMENT) || matches.is_present("C");
    let squeeze_flag = matches.is_present(options::SQUEEZE);
    let truncate_set1_flag = matches.is_present(options::TRUNCATE_SET1);

    let sets = matches
        .values_of(options::SETS)
        .map(|v| v.map(ToString::to_string).collect::<Vec<_>>())
        .unwrap_or_default();

    if sets.is_empty() {
        show_error!(
            "missing operand\nTry `{} --help` for more information.",
            executable!()
        );
        return 1;
    }

    if !(delete_flag || squeeze_flag) && sets.len() < 2 {
        show_error!(
            "missing operand after '{}'\nTry `{} --help` for more information.",
            sets[0],
            executable!()
        );
        return 1;
    }

    let stdin = stdin();
    let mut locked_stdin = stdin.lock();
    let stdout = stdout();
    let locked_stdout = stdout.lock();
    let mut buffered_stdout = BufWriter::new(locked_stdout);

    if delete_flag {
        if squeeze_flag {
            let mut delete_buffer = vec![];
            {
                let mut delete_writer = BufWriter::new(&mut delete_buffer);
                let delete_op =
                    DeleteOperation::new(Sequence::parse_set_string(&sets[0]), complement_flag);
                translate_input_new(&mut locked_stdin, &mut delete_writer, delete_op);
            }
            {
                let mut squeeze_reader = BufReader::new(delete_buffer.as_bytes());
                let squeeze_op =
                    SqueezeOperation::new(Sequence::parse_set_string(&sets[1]), complement_flag);
                translate_input_new(&mut squeeze_reader, &mut buffered_stdout, squeeze_op);
            }
        } else {
            let op = DeleteOperation::new(Sequence::parse_set_string(&sets[0]), complement_flag);
            translate_input_new(&mut locked_stdin, &mut buffered_stdout, op);
        }
    } else if squeeze_flag {
        if sets.len() < 2 {
            let op = SqueezeOperation::new(Sequence::parse_set_string(&sets[0]), complement_flag);
            translate_input_new(&mut locked_stdin, &mut buffered_stdout, op);
        } else {
            let mut translate_buffer = vec![];
            {
                let mut writer = BufWriter::new(&mut translate_buffer);
                let translate_op = TranslateOperation::new(
                    Sequence::parse_set_string(&sets[0]),
                    Sequence::parse_set_string(&sets[1]),
                    truncate_set1_flag,
                    complement_flag,
                );
                translate_input_new(&mut locked_stdin, &mut writer, translate_op);
            }
            {
                let mut reader = BufReader::new(translate_buffer.as_bytes());
                let squeeze_op = SqueezeOperation::new(Sequence::parse_set_string(&sets[1]), false);
                translate_input_new(&mut reader, &mut buffered_stdout, squeeze_op);
            }
        }
    } else {
        let op = TranslateOperation::new(
            Sequence::parse_set_string(&sets[0]),
            Sequence::parse_set_string(&sets[1]),
            truncate_set1_flag,
            complement_flag,
        );
        translate_input_new(&mut locked_stdin, &mut buffered_stdout, op);
    }

    0
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(executable!())
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::COMPLEMENT)
                // .visible_short_alias('C')  // TODO: requires clap "3.0.0-beta.2"
                .short("c")
                .long(options::COMPLEMENT)
                .help("use the complement of SET1"),
        )
        .arg(
            Arg::with_name("C") // work around for `Arg::visible_short_alias`
                .short("C")
                .help("same as -c"),
        )
        .arg(
            Arg::with_name(options::DELETE)
                .short("d")
                .long(options::DELETE)
                .help("delete characters in SET1, do not translate"),
        )
        .arg(
            Arg::with_name(options::SQUEEZE)
                .long(options::SQUEEZE)
                .short("s")
                .help(
                    "replace each sequence  of  a  repeated  character  that  is
  listed  in the last specified SET, with a single occurrence
  of that character",
                ),
        )
        .arg(
            Arg::with_name(options::TRUNCATE_SET1)
                .long(options::TRUNCATE_SET1)
                .short("t")
                .help("first truncate SET1 to length of SET2"),
        )
        .arg(
            Arg::with_name(options::SETS)
                .multiple(true)
                .takes_value(true)
                .min_values(1)
                .max_values(2),
        )
}
