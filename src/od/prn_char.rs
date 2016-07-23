
// TODO: multi-byte chars
// Quoth the man page: Multi-byte characters are displayed in the area corresponding to the first byte of the character. The remaining bytes are shown as `**'.

static A_CHRS : [&'static str; 160]  =
["nul",   "soh",   "stx",   "etx",   "eot",   "enq",   "ack",   "bel",
 "bs",    "ht",   "nl",     "vt",    "ff",    "cr",    "so",    "si",
 "dle",   "dc1",   "dc2",   "dc3",   "dc4",   "nak",   "syn",   "etb",
 "can",   "em",   "sub",   "esc",    "fs",    "gs",    "rs",    "us",
 "sp",     "!",     "\"",     "#",     "$",     "%",     "&",     "'",
  "(",     ")",     "*",     "+",     ",",     "-",     ".",     "/",
  "0",     "1",     "2",     "3",     "4",     "5",     "6",     "7",
  "8",     "9",     ":",     ";",     "<",     "=",     ">",     "?",
  "@",     "A",     "B",     "C",     "D",     "E",     "F",     "G",
  "H",     "I",     "J",     "K",     "L",     "M",     "N",     "O",
  "P",     "Q",     "R",     "S",     "T",     "U",     "V",     "W",
  "X",     "Y",     "Z",     "[",     "\\",    "]",     "^",     "_",
  "`",     "a",     "b",     "c",     "d",     "e",     "f",     "g",
  "h",     "i",     "j",     "k",     "l",     "m",     "n",     "o",
  "p",     "q",     "r",     "s",     "t",     "u",     "v",     "w",
  "x",     "y",     "z",     "{",     "|",     "}",     "~",   "del",
 "80",    "81",    "82",    "83",    "84",    "85",    "86",    "87",
 "88",    "89",    "8a",    "8b",    "8c",    "8d",    "8e",    "8f",
 "90",    "91",    "92",    "93",    "94",    "95",    "96",    "97",
 "98",    "99",    "9a",    "9b",    "9c",    "9d",    "9e",    "9f"];

pub fn print_item_a(p: u64, _: usize) {
    // itembytes == 1
    let b = (p & 0xff) as u8;
    print!("{:>4}", A_CHRS.get(b as usize).unwrap_or(&"?") // XXX od dose not actually do this, it just prints the byte
  );
}


static C_CHRS : [&'static str; 127]  = [
"\\0",   "001",   "002",   "003",   "004",   "005",   "006",    "\\a",
"\\b",    "\\t",  "\\n",   "\\v",    "\\f",    "\\r",   "016",   "017",
"020",   "021",   "022",   "023",   "024",   "025",   "026",   "027",
"030",   "031",   "032",   "033",   "034",   "035",   "036",   "037",
  " ",   "!",     "\"",     "#",     "$",     "%",     "&",     "'",
  "(",     ")",     "*",     "+",     ",",     "-",     ".",     "/",
  "0",     "1",     "2",     "3",     "4",     "5",     "6",     "7",
  "8",     "9",     ":",     ";",     "<",     "=",     ">",     "?",
  "@",     "A",     "B",     "C",     "D",     "E",     "F",     "G",
  "H",     "I",     "J",     "K",     "L",     "M",     "N",     "O",
  "P",     "Q",     "R",     "S",     "T",     "U",     "V",     "W",
  "X",     "Y",     "Z",     "[",     "\\",     "]",     "^",     "_",
  "`",     "a",     "b",     "c",     "d",     "e",     "f",     "g",
  "h",     "i",     "j",     "k",     "l",     "m",     "n",     "o",
  "p",     "q",     "r",     "s",     "t",     "u",     "v",     "w",
  "x",     "y",     "z",     "{",     "|",     "}",     "~" ];


pub fn print_item_c(p: u64, _: usize) {
    // itembytes == 1
    let b = (p & 0xff) as usize;

    if b < C_CHRS.len() {
        match C_CHRS.get(b as usize) {
            Some(s) => print!("{:>4}", s),
            None => print!("{:>4}", b),
        }
    }
}
