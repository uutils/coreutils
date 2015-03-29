#![crate_name = "join"]
#![feature(str_words)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Gianpaolo Branca <gianpi101@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use std::io::{BufRead, BufReader};
use std::env;
use std::fs::File;
use std::cmp::Ordering;

pub fn main() {

    let path1 = match env::args().nth(1) {
        Some(path1) => path1,
        None => panic!("some file paths are missing!"), };
    let path2 = match env::args().nth(2) {
        Some(path2) => path2,
        None => panic!("some file paths are missing!"), };

    let file1 = match File::open(&path1) {
        Ok(file1) => file1,
        Err(_) => panic!("could not open {}", path1), };
    let file2 = match File::open(&path2) {
        Ok(file2) => file2,
        Err(_) => panic!("could not open {}", path2), };
    
    let reader1 = BufReader::new(file1);
    let reader2 = BufReader::new(file2);
    
    let mut iter1 = reader1.lines();
    let mut iter2 = reader2.lines();
    
    let mut opt1 = iter1.next();
    let mut opt2 = iter2.next();
    
    while opt1.is_some() && opt2.is_some()
    {
        let str1 = opt1.clone().unwrap().unwrap();
        let str2 = opt2.clone().unwrap().unwrap();
    
        let line_tok1: Vec<_> = str1.words().collect();
        let line_tok2: Vec<_> = str2.words().collect();

        match line_tok1[0].cmp(line_tok2[0]) {
        
            Ordering::Equal   => { println!("{} {} {}", line_tok1[0], line_tok1[1], line_tok2[1]);
                                   opt1 = iter1.next(); 
                                   opt2 = iter2.next();}
            Ordering::Less    => { opt1 = iter1.next();}
            Ordering::Greater => { opt2 = iter2.next();}
        }
        
    }
}
