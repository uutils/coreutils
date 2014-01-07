#[crate_id(name="sleep", vers="1.0.0", author="Arcterus")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern mod extra;

use std::num;
use std::cast;
use std::os;
use std::io::{stderr, timer};
use extra::getopts::groups;

fn main() {
    let args = os::args();
    let program = args[0].clone();

    let opts = ~[
        groups::optflag("h", "help", "display this help and exit"),
        groups::optflag("V", "version", "output version information and exit")
    ];
    let matches = match groups::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => {
            writeln!(&mut stderr() as &mut Writer, "{}", f.to_err_msg())
            os::set_exit_status(1);
            return
        }
    };

    if matches.opt_present("help") {
        println("sleep 1.0.0");
        println("");
        println("Usage:");
        println!("  {0:s} NUMBER[SUFFIX]", program);
        println("or");
        println!("  {0:s} OPTION", program);
        println("");
        print(groups::usage("Pause for NUMBER seconds.  SUFFIX may be 's' for seconds (the default),
'm' for minutes, 'h' for hours or 'd' for days.  Unlike most implementations
that require NUMBER be an integer, here NUMBER may be an arbitrary floating
point number.  Given two or more arguments, pause for the amount of time
specified by the sum of their values.", opts));
    } else if matches.opt_present("version") {
        println("sleep 1.0.0");
    } else if matches.free.is_empty() {
        writeln!(&mut stderr() as &mut Writer, "Missing an argument");
        writeln!(&mut stderr() as &mut Writer,
                 "For help, try '{0:s} --help'", program);
        os::set_exit_status(1);
    } else {
        sleep(matches.free);
    }
}

fn sleep(args: &[~str]) {
    let mut failed = false;
    let sleep_time = args.iter().fold(0.0, |result, arg| {
        if failed {
            0.0
        } else {
            unsafe {
                let suffix_time = match match_suffix(cast::transmute(arg)) {
                    Ok(m) => m,
                    Err(f) => {
                        writeln!(&mut stderr() as &mut Writer, "{}", f);
                        failed = true;
                        os::set_exit_status(1);
                        0
                    }
                };
                let num =
                    if suffix_time == 0 {
                        0.0
                    } else {
                        match num::from_str_radix::<f64>((*arg), 10) {
                            Some(m) => m,
                            None => {
                                writeln!(&mut stderr() as &mut Writer,
                                        "Invalid time interval '{}'", *arg);
                                failed = true;
                                os::set_exit_status(1);
                                0.0
                            }
                        }
                    };
                result + num * suffix_time as f64
            }
        }
    });
    if !failed {
        timer::sleep((sleep_time * 1000.0) as u64);
    }
}

fn match_suffix(arg: &mut ~str) -> Result<int, ~str> {
    let result = match (*arg).pop_char() {
        's' | 'S' => Ok(1),
        'm' | 'M' => Ok(60),
        'h' | 'H' => Ok(60 * 60),
        'd' | 'D' => Ok(60 * 60 * 24),
        val => {
            (*arg).push_char(val);
            if !val.is_alphabetic() {
                return Ok(1)
            } else {
                return Err(format!("Invalid time interval '{}'", *arg))
            }
        }
    };
    result
}
