use crate::constants;
#[derive(Debug)]
pub enum Event<'a> {
    Data(&'a [u8]),
    Line,
}
/// Loops over the lines read from a BufRead.
/// # Arguments
/// * `input` the ReadBuf to read from
/// * `zero` whether to use 0u8 as a line delimiter
/// * `on_event` a closure receiving some bytes read in a slice, or
///     event signalling a line was just read.
///     this is guaranteed to be signalled *directly* after the
///     slice containing the (CR on win)LF / 0 is passed
///
///     Return whether to continue
pub fn walk_lines<F>(
    input: &mut impl std::io::BufRead,
    zero: bool,
    mut on_event: F,
) -> std::io::Result<()>
where
    F: FnMut(Event) -> std::io::Result<bool>,
{
    let mut buffer = [0u8; constants::BUF_SIZE];
    loop {
        let read = loop {
            match input.read(&mut buffer) {
                Ok(n) => break n,
                Err(e) => match e.kind() {
                    std::io::ErrorKind::Interrupted => {}
                    _ => return Err(e),
                },
            }
        };
        if read == 0 {
            return Ok(());
        }
        let mut base = 0usize;
        for (i, byte) in buffer[..read].iter().enumerate() {
            match byte {
                b'\n' if !zero => {
                    on_event(Event::Data(&buffer[base..=i]))?;
                    base = i + 1;
                    if !on_event(Event::Line)? {
                        return Ok(());
                    }
                }
                0u8 if zero => {
                    on_event(Event::Data(&buffer[base..=i]))?;
                    base = i + 1;
                    if !on_event(Event::Line)? {
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
        on_event(Event::Data(&buffer[base..read]))?;
    }
}
