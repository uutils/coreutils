// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore btotal sigval
//! Read and write progress tracking for dd.
//!
//! The [`ProgUpdate`] struct represents summary statistics for the
//! read and write progress of a running `dd` process. The
//! [`gen_prog_updater`] function can be used to implement a progress
//! updater that runs in its own thread.
use std::io::Write;
use std::sync::mpsc;
#[cfg(target_os = "linux")]
use std::thread::JoinHandle;
use std::time::Duration;

#[cfg(target_os = "linux")]
use signal_hook::iterator::Handle;
use uucore::{
    error::UResult,
    format::num_format::{FloatVariant, Formatter},
};

use crate::numbers::{to_magnitude_and_suffix, SuffixType};

#[derive(PartialEq, Eq)]
pub(crate) enum ProgUpdateType {
    Periodic,
    Signal,
    Final,
}

/// Summary statistics for read and write progress of dd for a given duration.
pub(crate) struct ProgUpdate {
    /// Read statistics.
    ///
    /// This contains information about the number of blocks read from
    /// the data source.
    pub(crate) read_stat: ReadStat,

    /// Write statistics.
    ///
    /// This contains information about the number of blocks and
    /// number of bytes written to the data sink.
    pub(crate) write_stat: WriteStat,

    /// The time period over which the reads and writes were measured.
    pub(crate) duration: Duration,

    /// The status of the write.
    ///
    /// True if the write is completed, false if still in-progress.
    pub(crate) update_type: ProgUpdateType,
}

impl ProgUpdate {
    /// Instantiate this struct.
    pub(crate) fn new(
        read_stat: ReadStat,
        write_stat: WriteStat,
        duration: Duration,
        update_type: ProgUpdateType,
    ) -> Self {
        Self {
            read_stat,
            write_stat,
            duration,
            update_type,
        }
    }

    /// Write the number of complete and partial records both read and written.
    ///
    /// The information is written to `w`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::io::Cursor;
    /// use std::time::Duration;
    /// use crate::progress::{ProgUpdate, ReadStat, WriteStat};
    ///
    /// let read_stat = ReadStat::new(1, 2, 3, 999);
    /// let write_stat = WriteStat::new(4, 5, 6);
    /// let duration = Duration::new(789, 0);
    /// let prog_update = ProgUpdate {
    ///     read_stat,
    ///     write_stat,
    ///     duration,
    /// };
    ///
    /// let mut cursor = Cursor::new(vec![]);
    /// prog_update.write_io_lines(&mut cursor).unwrap();
    /// assert_eq!(
    ///     cursor.get_ref(),
    ///     b"1+2 records in\n3 truncated records\n4+5 records out\n"
    /// );
    /// ```
    fn write_io_lines(&self, w: &mut impl Write) -> std::io::Result<()> {
        self.read_stat.report(w)?;
        self.write_stat.report(w)?;
        match self.read_stat.records_truncated {
            0 => {}
            1 => writeln!(w, "1 truncated record")?,
            n => writeln!(w, "{n} truncated records")?,
        }
        Ok(())
    }

    /// Write the number of bytes written, duration, and throughput.
    ///
    /// The information is written to `w`. If `rewrite` is `true`,
    /// then a `\r` character is written first and no newline is
    /// written at the end. When writing to `stderr`, this has the
    /// visual effect of overwriting the previous characters on the
    /// line.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::io::Cursor;
    /// use std::time::Duration;
    /// use crate::progress::ProgUpdate;
    ///
    /// let prog_update = ProgUpdate {
    ///     read_stat: Default::default(),
    ///     write_stat: Default::default(),
    ///     duration: Duration::new(1, 0),  // one second
    /// };
    ///
    /// let mut cursor = Cursor::new(vec![]);
    /// let rewrite = false;
    /// prog_update.write_prog_line(&mut cursor, rewrite).unwrap();
    /// assert_eq!(cursor.get_ref(), b"0 bytes copied, 1.0 s, 0.0 B/s\n");
    /// ```
    fn write_prog_line(&self, w: &mut impl Write, rewrite: bool) -> UResult<()> {
        // The total number of bytes written as a string, in SI and IEC format.
        let btotal = self.write_stat.bytes_total;
        let btotal_metric = to_magnitude_and_suffix(btotal, SuffixType::Si);
        let btotal_bin = to_magnitude_and_suffix(btotal, SuffixType::Iec);

        // Compute the throughput (bytes per second) as a string.
        let duration = self.duration.as_secs_f64();
        let safe_millis = std::cmp::max(1, self.duration.as_millis());
        let rate = 1000 * (btotal / safe_millis);
        let transfer_rate = to_magnitude_and_suffix(rate, SuffixType::Si);

        // If we are rewriting the progress line, do write a carriage
        // return (`\r`) at the beginning and don't write a newline
        // (`\n`) at the end.
        let (carriage_return, newline) = if rewrite { ("\r", "") } else { ("", "\n") };

        // The duration should be formatted as in `printf %g`.
        let mut duration_str = Vec::new();
        uucore::format::num_format::Float {
            variant: FloatVariant::Shortest,
            ..Default::default()
        }
        .fmt(&mut duration_str, duration)?;
        // We assume that printf will output valid UTF-8
        let duration_str = std::str::from_utf8(&duration_str).unwrap();

        // If the number of bytes written is sufficiently large, then
        // print a more concise representation of the number, like
        // "1.2 kB" and "1.0 KiB".
        match btotal {
            1 => write!(
                w,
                "{carriage_return}{btotal} byte copied, {duration_str} s, {transfer_rate}/s{newline}",
            )?,
            0..=999 => write!(
                w,
                "{carriage_return}{btotal} bytes copied, {duration_str} s, {transfer_rate}/s{newline}",
            )?,
            1000..=1023 => write!(
                w,
                "{carriage_return}{btotal} bytes ({btotal_metric}) copied, {duration_str} s, {transfer_rate}/s{newline}",
            )?,
            _ => write!(
                w,
                "{carriage_return}{btotal} bytes ({btotal_metric}, {btotal_bin}) copied, {duration_str} s, {transfer_rate}/s{newline}",
            )?,
        };
        Ok(())
    }

    /// Write all summary statistics.
    ///
    /// This is a convenience method that calls
    /// [`ProgUpdate::write_io_lines`] and
    /// [`ProgUpdate::write_prog_line`] in that order. The information
    /// is written to `w`. It optionally begins by writing a new line,
    /// intended to handle the case of an existing progress line.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::io::Cursor;
    /// use std::time::Duration;
    /// use crate::progress::ProgUpdate;
    ///
    /// let prog_update = ProgUpdate {
    ///     read_stat: Default::default(),
    ///     write_stat: Default::default(),
    ///     duration: Duration::new(1, 0), // one second
    /// };
    /// let mut cursor = Cursor::new(vec![]);
    /// prog_update.write_transfer_stats(&mut cursor, false).unwrap();
    /// let mut iter = cursor.get_ref().split(|v| *v == b'\n');
    /// assert_eq!(iter.next().unwrap(), b"0+0 records in");
    /// assert_eq!(iter.next().unwrap(), b"0+0 records out");
    /// assert_eq!(iter.next().unwrap(), b"0 bytes copied, 1.0 s, 0.0 B/s");
    /// assert_eq!(iter.next().unwrap(), b"");
    /// assert!(iter.next().is_none());
    /// ```
    fn write_transfer_stats(&self, w: &mut impl Write, new_line: bool) -> UResult<()> {
        if new_line {
            writeln!(w)?;
        }
        self.write_io_lines(w)?;
        let rewrite = false;
        self.write_prog_line(w, rewrite)?;
        Ok(())
    }

    /// Print number of complete and partial records read and written to stderr.
    ///
    /// See [`ProgUpdate::write_io_lines`] for more information.
    pub(crate) fn print_io_lines(&self) {
        let mut stderr = std::io::stderr();
        self.write_io_lines(&mut stderr).unwrap();
    }

    /// Re-print the number of bytes written, duration, and throughput.
    ///
    /// See [`ProgUpdate::write_prog_line`] for more information.
    pub(crate) fn reprint_prog_line(&self) {
        let mut stderr = std::io::stderr();
        let rewrite = true;
        self.write_prog_line(&mut stderr, rewrite).unwrap();
    }

    /// Write all summary statistics.
    ///
    /// See [`ProgUpdate::write_transfer_stats`] for more information.
    pub(crate) fn print_transfer_stats(&self, new_line: bool) {
        let mut stderr = std::io::stderr();
        self.write_transfer_stats(&mut stderr, new_line).unwrap();
    }

    /// Write all the final statistics.
    pub(crate) fn print_final_stats(
        &self,
        print_level: Option<StatusLevel>,
        progress_printed: bool,
    ) {
        match print_level {
            Some(StatusLevel::None) => {}
            Some(StatusLevel::Noxfer) => self.print_io_lines(),
            Some(StatusLevel::Progress) | None => self.print_transfer_stats(progress_printed),
        }
    }
}

/// Read statistics.
///
/// This contains information about the number of blocks read from the
/// input file. A block is sometimes referred to as a "record".
#[derive(Clone, Copy, Default)]
pub(crate) struct ReadStat {
    /// The number of complete blocks that have been read.
    pub(crate) reads_complete: u64,

    /// The number of partial blocks that have been read.
    ///
    /// A partial block read can happen if, for example, there are
    /// fewer bytes in the input file than the specified input block
    /// size.
    pub(crate) reads_partial: u64,

    /// The number of truncated records.
    ///
    /// A truncated record can only occur in `conv=block` mode.
    pub(crate) records_truncated: u32,

    /// The total number of bytes read.
    pub(crate) bytes_total: u64,
}

impl ReadStat {
    /// Create a new instance.
    #[allow(dead_code)]
    fn new(complete: u64, partial: u64, truncated: u32, bytes_total: u64) -> Self {
        Self {
            reads_complete: complete,
            reads_partial: partial,
            records_truncated: truncated,
            bytes_total,
        }
    }

    /// Whether this counter has zero complete reads and zero partial reads.
    pub(crate) fn is_empty(&self) -> bool {
        self.reads_complete == 0 && self.reads_partial == 0
    }

    /// Write the counts in the format required by `dd`.
    ///
    /// # Errors
    ///
    /// If there is a problem writing to `w`.
    fn report(&self, w: &mut impl Write) -> std::io::Result<()> {
        writeln!(
            w,
            "{}+{} records in",
            self.reads_complete, self.reads_partial
        )?;
        Ok(())
    }
}

impl std::ops::AddAssign for ReadStat {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            reads_complete: self.reads_complete + other.reads_complete,
            reads_partial: self.reads_partial + other.reads_partial,
            records_truncated: self.records_truncated + other.records_truncated,
            bytes_total: self.bytes_total + other.bytes_total,
        }
    }
}

/// Write statistics.
///
/// This contains information about the number of blocks written to
/// the output file and the total number of bytes written.
#[derive(Clone, Copy, Default)]
pub(crate) struct WriteStat {
    /// The number of complete blocks that have been written.
    pub(crate) writes_complete: u64,

    /// The number of partial blocks that have been written.
    ///
    /// A partial block write can happen if, for example, there are
    /// fewer bytes in the input file than the specified output block
    /// size.
    pub(crate) writes_partial: u64,

    /// The total number of bytes written.
    pub(crate) bytes_total: u128,
}

impl WriteStat {
    /// Create a new instance.
    #[allow(dead_code)]
    fn new(complete: u64, partial: u64, bytes_total: u128) -> Self {
        Self {
            writes_complete: complete,
            writes_partial: partial,
            bytes_total,
        }
    }

    /// Write the counts in the format required by `dd`.
    ///
    /// # Errors
    ///
    /// If there is a problem writing to `w`.
    fn report(&self, w: &mut impl Write) -> std::io::Result<()> {
        writeln!(
            w,
            "{}+{} records out",
            self.writes_complete, self.writes_partial
        )
    }
}

impl std::ops::AddAssign for WriteStat {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            writes_complete: self.writes_complete + other.writes_complete,
            writes_partial: self.writes_partial + other.writes_partial,
            bytes_total: self.bytes_total + other.bytes_total,
        }
    }
}

impl std::ops::Add for WriteStat {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            writes_complete: self.writes_complete + other.writes_complete,
            writes_partial: self.writes_partial + other.writes_partial,
            bytes_total: self.bytes_total + other.bytes_total,
        }
    }
}

/// How much detail to report when printing transfer statistics.
///
/// This corresponds to the available settings of the `status`
/// command-line argument.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum StatusLevel {
    /// Report number of blocks read and written, throughput, and volume.
    ///
    /// This corresponds to `status=progress`.
    Progress,

    /// Report number of blocks read and written, but no throughput and volume.
    ///
    /// This corresponds to `status=noxfer`.
    Noxfer,

    /// Print no status information.
    None,
}

/// Return a closure that can be used in its own thread to print progress info.
///
/// This function returns a closure that receives [`ProgUpdate`]
/// instances sent through `rx`. When a [`ProgUpdate`] instance is
/// received, the transfer statistics are re-printed to stderr.
#[cfg(not(target_os = "linux"))]
pub(crate) fn gen_prog_updater(
    rx: mpsc::Receiver<ProgUpdate>,
    print_level: Option<StatusLevel>,
) -> impl Fn() {
    move || {
        let mut progress_printed = false;
        while let Ok(update) = rx.recv() {
            // Print the final read/write statistics.
            if update.update_type == ProgUpdateType::Final {
                update.print_final_stats(print_level, progress_printed);
                return;
            }
            if Some(StatusLevel::Progress) == print_level {
                update.reprint_prog_line();
                progress_printed = true;
            }
        }
    }
}

/// signal handler listens for SIGUSR1 signal and runs provided closure.
#[cfg(target_os = "linux")]
pub(crate) struct SignalHandler {
    handle: Handle,
    thread: Option<JoinHandle<()>>,
}

#[cfg(target_os = "linux")]
impl SignalHandler {
    pub(crate) fn install_signal_handler(
        f: Box<dyn Send + Sync + Fn()>,
    ) -> Result<Self, std::io::Error> {
        use signal_hook::consts::signal::*;
        use signal_hook::iterator::Signals;

        let mut signals = Signals::new([SIGUSR1])?;
        let handle = signals.handle();
        let thread = std::thread::spawn(move || {
            for signal in &mut signals {
                match signal {
                    SIGUSR1 => (*f)(),
                    _ => unreachable!(),
                }
            }
        });

        Ok(Self {
            handle,
            thread: Some(thread),
        })
    }
}

#[cfg(target_os = "linux")]
impl Drop for SignalHandler {
    fn drop(&mut self) {
        self.handle.close();
        if let Some(thread) = std::mem::take(&mut self.thread) {
            thread.join().unwrap();
        }
    }
}

/// Return a closure that can be used in its own thread to print progress info.
///
/// This function returns a closure that receives [`ProgUpdate`]
/// instances sent through `rx`. When a [`ProgUpdate`] instance is
/// received, the transfer statistics are re-printed to stderr.
///
/// The closure also registers a signal handler for `SIGUSR1`. When
/// the `SIGUSR1` signal is sent to this process, the transfer
/// statistics are printed to stderr.
#[cfg(target_os = "linux")]
pub(crate) fn gen_prog_updater(
    rx: mpsc::Receiver<ProgUpdate>,
    print_level: Option<StatusLevel>,
) -> impl Fn() {
    // --------------------------------------------------------------
    move || {
        // Holds the state of whether we have printed the current progress.
        // This is needed so that we know whether or not to print a newline
        // character before outputting non-progress data.
        let mut progress_printed = false;
        while let Ok(update) = rx.recv() {
            match update.update_type {
                ProgUpdateType::Final => {
                    // Print the final read/write statistics.
                    update.print_final_stats(print_level, progress_printed);
                    return;
                }
                ProgUpdateType::Periodic => {
                    // (Re)print status line if progress is requested.
                    if Some(StatusLevel::Progress) == print_level {
                        update.reprint_prog_line();
                        progress_printed = true;
                    }
                }
                ProgUpdateType::Signal => {
                    update.print_transfer_stats(progress_printed);
                    // Reset the progress printed, since print_transfer_stats always prints a newline.
                    progress_printed = false;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::io::Cursor;
    use std::time::Duration;

    use super::{ProgUpdate, ReadStat, WriteStat};

    fn prog_update_write(n: u128) -> ProgUpdate {
        ProgUpdate {
            read_stat: ReadStat::default(),
            write_stat: WriteStat {
                bytes_total: n,
                ..Default::default()
            },
            duration: Duration::new(1, 0), // one second
            update_type: super::ProgUpdateType::Periodic,
        }
    }

    fn prog_update_duration(duration: Duration) -> ProgUpdate {
        ProgUpdate {
            read_stat: ReadStat::default(),
            write_stat: WriteStat::default(),
            duration,
            update_type: super::ProgUpdateType::Periodic,
        }
    }

    #[test]
    fn test_read_stat_report() {
        let read_stat = ReadStat::new(1, 2, 3, 4);
        let mut cursor = Cursor::new(vec![]);
        read_stat.report(&mut cursor).unwrap();
        assert_eq!(cursor.get_ref(), b"1+2 records in\n");
    }

    #[test]
    fn test_write_stat_report() {
        let write_stat = WriteStat::new(1, 2, 3);
        let mut cursor = Cursor::new(vec![]);
        write_stat.report(&mut cursor).unwrap();
        assert_eq!(cursor.get_ref(), b"1+2 records out\n");
    }

    #[test]
    fn test_prog_update_write_io_lines() {
        let read_stat = ReadStat::new(1, 2, 3, 4);
        let write_stat = WriteStat::new(4, 5, 6);
        let duration = Duration::new(789, 0);
        let update_type = super::ProgUpdateType::Periodic;
        let prog_update = ProgUpdate {
            read_stat,
            write_stat,
            duration,
            update_type,
        };

        let mut cursor = Cursor::new(vec![]);
        prog_update.write_io_lines(&mut cursor).unwrap();
        assert_eq!(
            cursor.get_ref(),
            b"1+2 records in\n4+5 records out\n3 truncated records\n"
        );
    }

    #[test]
    fn test_prog_update_write_prog_line() {
        let prog_update = ProgUpdate {
            read_stat: ReadStat::default(),
            write_stat: WriteStat::default(),
            duration: Duration::new(1, 0), // one second
            update_type: super::ProgUpdateType::Periodic,
        };

        let mut cursor = Cursor::new(vec![]);
        let rewrite = false;
        prog_update.write_prog_line(&mut cursor, rewrite).unwrap();
        // TODO The expected output string below is what our code
        // produces today, but it does not match GNU dd:
        //
        //     $ : | dd
        //     0 bytes copied, 7.9151e-05 s, 0.0 kB/s
        //
        // The throughput still does not match GNU dd.
        assert_eq!(cursor.get_ref(), b"0 bytes copied, 1 s, 0.0 B/s\n");

        let prog_update = prog_update_write(1);
        let mut cursor = Cursor::new(vec![]);
        prog_update.write_prog_line(&mut cursor, rewrite).unwrap();
        assert_eq!(cursor.get_ref(), b"1 byte copied, 1 s, 0.0 B/s\n");

        let prog_update = prog_update_write(999);
        let mut cursor = Cursor::new(vec![]);
        prog_update.write_prog_line(&mut cursor, rewrite).unwrap();
        assert_eq!(cursor.get_ref(), b"999 bytes copied, 1 s, 0.0 B/s\n");

        let prog_update = prog_update_write(1000);
        let mut cursor = Cursor::new(vec![]);
        prog_update.write_prog_line(&mut cursor, rewrite).unwrap();
        assert_eq!(
            cursor.get_ref(),
            b"1000 bytes (1.0 kB) copied, 1 s, 1.0 kB/s\n"
        );

        let prog_update = prog_update_write(1023);
        let mut cursor = Cursor::new(vec![]);
        prog_update.write_prog_line(&mut cursor, rewrite).unwrap();
        assert_eq!(
            cursor.get_ref(),
            b"1023 bytes (1.0 kB) copied, 1 s, 1.0 kB/s\n"
        );

        let prog_update = prog_update_write(1024);
        let mut cursor = Cursor::new(vec![]);
        prog_update.write_prog_line(&mut cursor, rewrite).unwrap();
        assert_eq!(
            cursor.get_ref(),
            b"1024 bytes (1.0 kB, 1.0 KiB) copied, 1 s, 1.0 kB/s\n"
        );
    }

    #[test]
    fn write_transfer_stats() {
        let prog_update = ProgUpdate {
            read_stat: ReadStat::default(),
            write_stat: WriteStat::default(),
            duration: Duration::new(1, 0), // one second
            update_type: super::ProgUpdateType::Periodic,
        };
        let mut cursor = Cursor::new(vec![]);
        prog_update
            .write_transfer_stats(&mut cursor, false)
            .unwrap();
        let mut iter = cursor.get_ref().split(|v| *v == b'\n');
        assert_eq!(iter.next().unwrap(), b"0+0 records in");
        assert_eq!(iter.next().unwrap(), b"0+0 records out");
        assert_eq!(iter.next().unwrap(), b"0 bytes copied, 1 s, 0.0 B/s");
        assert_eq!(iter.next().unwrap(), b"");
        assert!(iter.next().is_none());
    }

    #[test]
    fn write_final_transfer_stats() {
        // Tests the formatting of the final statistics written after a progress line.
        let prog_update = ProgUpdate {
            read_stat: ReadStat::default(),
            write_stat: WriteStat::default(),
            duration: Duration::new(1, 0), // one second
            update_type: super::ProgUpdateType::Periodic,
        };
        let mut cursor = Cursor::new(vec![]);
        let rewrite = true;
        prog_update.write_prog_line(&mut cursor, rewrite).unwrap();
        prog_update.write_transfer_stats(&mut cursor, true).unwrap();
        let mut iter = cursor.get_ref().split(|v| *v == b'\n');
        assert_eq!(iter.next().unwrap(), b"\r0 bytes copied, 1 s, 0.0 B/s");
        assert_eq!(iter.next().unwrap(), b"0+0 records in");
        assert_eq!(iter.next().unwrap(), b"0+0 records out");
        assert_eq!(iter.next().unwrap(), b"0 bytes copied, 1 s, 0.0 B/s");
        assert_eq!(iter.next().unwrap(), b"");
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_duration_precision() {
        let prog_update = prog_update_duration(Duration::from_nanos(123));
        let mut cursor = Cursor::new(vec![]);
        let rewrite = false;
        prog_update.write_prog_line(&mut cursor, rewrite).unwrap();
        assert_eq!(cursor.get_ref(), b"0 bytes copied, 1.23e-07 s, 0.0 B/s\n");
    }
}
