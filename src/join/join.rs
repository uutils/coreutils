#![crate_name = "join"]

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
extern crate getopts;
use getopts::Options;

static VERSION: &'static str = "1.0.0";

pub fn main() {

    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();
    opts.optflag("V", "version", "print the version of the program");
    opts.optflag("h", "help", "print this help menu");
    opts.optflag("i", "ignore-case", "ignore differences in case when comparing fileds");
    opts.optmulti("a", "all", "print also non joinable lines from FILE1 or FILE2", "-a 1 -a 2");
    opts.optopt("t", "", "use CHAR as an input and output filed separator", "-t -");
    
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) } };
    
    if matches.opt_present("h") {
        println!("coming soon!");
        return; }
    
    if matches.opt_present("V") {
        println!("join {} written by Gianpaolo Branca", VERSION);
        return; }
    
    let mut flag_all1: bool = false;
    let mut flag_all2: bool = false;
    for strs in matches.opt_strs("a") {
        match strs.as_ref() {
            "1" => flag_all1 = true,
            "2" => flag_all2 = true, 
             _  => { println!("invalid argument"); return}, } }
             
    let flag_t: char = match matches.opt_str("t").as_ref() {
        Some(t) => {
            if t.len() > 1 { println!("invalid multichar tabulator: {}", t); return}
            t.clone().pop().unwrap() },
        None => ' ', };

    let file1 = match File::open(&matches.free[0]) {
        Ok(file1) => file1,
        Err(_) => panic!("could not open {}"), };
    let file2 = match File::open(&matches.free[1]) {
        Ok(file2) => file2,
        Err(_) => panic!("could not open {}"), };
    
    let mut iter1 = BufReader::new(file1).lines();
    let mut iter2 = BufReader::new(file2).lines();
    
    let mut opt1 = iter1.next();
    let mut opt2 = iter2.next();
    
    while opt1.is_some() && opt2.is_some() {
    
        let str1 = opt1.as_ref().unwrap().as_ref().unwrap().clone();
        let str2 = opt2.as_ref().unwrap().as_ref().unwrap().clone();
    
        let line_tok1 = str1.split(flag_t).collect::<Vec<&str>>();
        let line_tok2 = str2.split(flag_t).collect::<Vec<&str>>();
        
        match line_tok1[0].cmp(line_tok2[0]) {
        
            Ordering::Equal   => { 
                println!("{}{}{}{}{}", line_tok1[0], flag_t, line_tok1[1], flag_t, line_tok2[1]);
                opt1 = iter1.next();
                opt2 = iter2.next();},
            Ordering::Less    => {
                if flag_all1 { println!("{}{}{}", line_tok1[0], flag_t, line_tok1[1]) };
                opt1 = iter1.next();},
            Ordering::Greater => {
                if flag_all2 { println!("{}{}{}", line_tok2[0], flag_t, line_tok2[1]) };
                opt2 = iter2.next();}, } }
    
    while flag_all1 && opt1.is_some() {
        let str1 = opt1.as_ref().unwrap().as_ref().unwrap().clone();
        let line_tok1 = str1.split(flag_t).collect::<Vec<&str>>();
        println!("{}{}{}", line_tok1[0], flag_t, line_tok1[1]); 
        opt1 = iter1.next(); }
        
    while flag_all2 && opt2.is_some() {
        let str2 = opt2.as_ref().unwrap().as_ref().unwrap().clone();
        let line_tok2 = str2.split(flag_t).collect::<Vec<&str>>();
        println!("{}{}{}", line_tok2[0], flag_t, line_tok2[1]); 
        opt2 = iter2.next(); }
}
