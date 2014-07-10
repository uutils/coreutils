#![crate_name = "od"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Ben Hirsch <benhirsch24@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
extern crate collections;

use collections::string::String;
use std::slice::bytes;

use std::io::File;

#[deriving(Show)]
enum Radix { Decimal, Hexadecimal, Octal, Binary }

pub fn uumain(args: Vec<String>) -> int {
   let opts = [
      getopts::optopt("A", "address-radix", "Select the base in which file offsets are printed.", "RADIX"),
      getopts::optopt("j", "skip-bytes", "Skip bytes input bytes before formatting and writing.", "BYTES"),
   ];

   let matches = match getopts::getopts(args.tail(), opts) {
      Ok(m) => m,
      Err(f) => fail!("Invalid options\n{}", f)
   };


   let mut rad = Octal;
   if matches.opt_present("A") {
      rad = parse_radix(matches.opt_str("A"));
   } else {
      println!("{}", getopts::usage("od", opts));
   }

   let mut fname;
   match args.last() {
      Some(n) => fname = n,
      None    => { fail!("Need fname for now") ; }
   };

   main(rad, fname.clone());

   0
}

fn main(radix: Radix, fname: String) {
   let mut f = match File::open(&Path::new(fname)) {
      Ok(f) => f,
      Err(e) => fail!("file error: {}", e)
   };

   let mut addr = 0;
   let mut bytes = [0, .. 16];
   loop {
      match f.read(bytes) {
         Ok(n) => {
            print!("{:07o}", addr);
            match radix {
               Decimal => { 
               },
               Octal => {
                  for b in range(0, n/std::u16::BYTES) {
                     let bs = bytes.slice(2*b, 2*b+2);
                     let p: u16 = bs[1] as u16 << 8 | bs[0] as u16;
                     print!(" {:06o}", p);
                  }
                  if n % std::u16::BYTES == 1 {
                     print!(" {:06o}", bytes[n-1]);
                  }
               }
               _ => { }
            };
            print!("\n");
            addr += n;
         },
         Err(_) => { println!("{:07o}", addr); break; }
      };
   };
}

fn parse_radix(radix_str: Option<String>) -> Radix {
   let rad = match radix_str {
      Some(s) => {
         let st = s.into_bytes();
         if st.len() != 1 {
            fail!("Radix must be one of [d, o, b, x]\n");
         }

         let radix: char = *st.get(0) as char;
         if radix == 'd' {
            Decimal
         } else if radix == 'x' {
            Hexadecimal
         } else if radix == 'o' {
            Octal
         } else if radix == 'b' {
            Binary
         } else {
            fail!("Radix must be one of [d, o, b, x]\n");
         }
      },
      None => Octal
   };

   rad
}
