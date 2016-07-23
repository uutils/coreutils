use std;
use std::io;
use std::io::BufReader;
use std::fs::File;
use std::io::Write;

#[derive(Debug)]
pub enum InputSource<'a> {
    FileName(&'a str ),
    Stdin
}

// MultifileReader - concatenate all our input, file or stdin.
pub struct MultifileReader<'a> {
    ni: std::slice::Iter<'a, InputSource<'a>>,
    curr_file: Option<Box<io::Read>>,
    pub any_err: bool,
}

impl<'b> MultifileReader<'b> {
    pub fn new<'a>(fnames: &'a [InputSource]) -> MultifileReader<'a> {
        let mut mf = MultifileReader {
            ni: fnames.iter(),
            curr_file: None, // normally this means done; call next_file()
            any_err: false,
        };
        mf.next_file();
        return mf;
    }

    fn next_file(&mut self) {
        // loop retries with subsequent files if err - normally 'loops' once
        loop {
            match self.ni.next() {
                None => {
                    self.curr_file = None;
                    return;
                }
                Some(input) => {
                    match *input {
                        InputSource::Stdin => {
                            self.curr_file = Some(Box::new(BufReader::new(std::io::stdin())));
                            return;
                        }
                        InputSource::FileName(fname) => {
                            match File::open(fname) {
                                Ok(f) => {
                                    self.curr_file = Some(Box::new(BufReader::new(f)));
                                    return;
                                }
                                Err(e) => {
                                    // If any file can't be opened,
                                    // print an error at the time that the file is needed,
                                    // then move on the the next file.
                                    // This matches the behavior of the original `od`
                                    let _ =
                                        writeln!(&mut std::io::stderr(), "od: '{}': {}", fname, e);
                                    self.any_err = true
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Fill buf with bytes read from the list of files
    // Returns Ok(<number of bytes read>)
    // Handles io errors itself, thus always returns OK
    // Fills the provided buffer completely, unless it has run out of input.
    // If any call returns short (< buf.len()), all subsequent calls will return Ok<0>
    pub fn f_read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut xfrd = 0;
        // while buffer we are filling is not full.. May go thru several files.
        'fillloop: while xfrd < buf.len() {
            match self.curr_file {
                None => break,
                Some(ref mut curr_file) => {
                    loop {
                        // stdin may return on 'return' (enter), even though the buffer isn't full.
                        xfrd += match curr_file.read(&mut buf[xfrd..]) {
                            Ok(0) => break,
                            Ok(n) => n,
                            Err(e) => panic!("file error: {}", e),
                        };
                        if xfrd == buf.len() {
                            // transferred all that was asked for.
                            break 'fillloop;
                        }
                    }
                }
            }
            self.next_file();
        }
        Ok(xfrd)
    }
}
