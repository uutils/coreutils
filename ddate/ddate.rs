/*
 * This file is part of the uutils coreutils pacakge.
 *
 * (c) Graham Watt <gmwatt@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/*
 * Currently ddate only prints out the current date in erisian format,
 * just like ddate from util-linux called with no arguments.
 * TODO: Format parsing, date specification. 
 */
extern mod extra;

use extra::time;
use std::i32::{min_value, max_value};

struct erisian_date {
	year:	i32,
	day:	i32,
	tibsday:bool,
}

impl ToStr for erisian_date {
	fn to_str(&self) -> ~str {
		let seasons = ["Chaos", "Discord", "Confusion", "Bureaucracy", "The Aftermath"];
		let days = ["Sweetmorn", "Boomtime", "Pungendat", "Prickle-Prickle", "Setting Orange"];
		if self.tibsday {
			format!("Today is St. Tib's Day in the YOLD {:d}\n", self.year)
		} else {
			format!("Today is {:s}, the {:s} day of {:s} in the YOLD {:d}\n",
				days[self.day % 5], dayToStr(1 + self.day % 73),
				 seasons[self.day / 73], self.year)
		}
	}
}

fn dayToStr(day: i32) -> ~str {
	match (day, day % 10) {
		(-1, _) => ~"",
		(11, _) => ~"11th",
		(12, _) => ~"12th",
		(_, 1)  => day.to_str() + "st",
		(_, 2)  => day.to_str() + "nd",
		(_, 3)  => day.to_str() + "rd",
		_	=> day.to_str() + "th"
	}
}

fn leapyear(yr: i32) -> bool {
	yr % 4 == 0 && ((yr % 100 == 0) == (yr % 400 == 0))
}

fn ddate(tm: &time::Tm) -> erisian_date {
	let tibsday = 60;
	let mut date = erisian_date{year: tm.tm_year + 3066, day:-1, tibsday:false};
	match (leapyear(tm.tm_year), tm.tm_yday - tibsday) {
		(true, min_value .. -1) => date.day = tm.tm_yday,
		(true, 0) => date.tibsday = true,
		(true, 1 .. max_value) => date.day = tm.tm_yday - 1,
		(false, _) => date.day = tm.tm_yday,
		_ => {},
	}
	date
}

fn main() {
	println(ddate(&time::now()).to_str());
}
