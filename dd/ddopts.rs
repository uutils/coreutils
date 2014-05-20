/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Colin Warren <me@zv.ms>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: dd (GNU coreutils) 8.21 */

/**
 * dd has a custom command line flag syntax in the form key=value
 *
 * keys may contain any lower case english letter (a-z)
 * values may contain any character
 * values (depending on the key) may be:
 *   * unsigned integers in decimal form
 *   * human-readable byte representations
 *   * paths to a file or device in the filesystem
 *   * comma-separated lists of flags
 */

use std::collections::hashmap::{HashMap,HashSet};

pub struct Opts {
    unknown_keys: HashSet<String>,
    values: HashMap<String, String>,
}

impl Opts {
    pub fn new() -> Opts {
        Opts {
            unknown_keys: HashSet::new(),
            values: HashMap::new(),
        }
    }

    pub fn parse(&mut self, args: Vec<String>) {
        for arg in args.iter() {
            let mut key = String::new();
            let mut value = String::new();

            let mut in_key = true;
            for chr in arg.as_slice().chars() {
                if in_key {
                    if chr == '=' {
                        in_key = false;
                        continue;
                    }
                    key.push_char(chr);
                } else {
                    value.push_char(chr);
                }
            }

            self.unknown_keys.insert(key.clone());

            self.values.insert(key, value);
        }
    }

    pub fn ok(&self) -> bool {
        self.unknown_keys.len() == 0
    }

    pub fn errors(&self) -> Vec<String> {
        self.unknown_keys.iter()
            .map(|key| format!("unrecognized operand '{}'", key))
            .collect()
    }

    pub fn get(&mut self, key: &str) -> Option<String> {
        self.unknown_keys.remove(&key.to_string());

        match self.values.find(&key.to_string()) {
            Some(s) => Some(s.clone()),
            None => None
        }
    }
}
