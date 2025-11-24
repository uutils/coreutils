// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Set of functions to parsing TTY
use std::{
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Teletype {
    Tty(u64),
    TtyS(u64),
    Pts(u64),
    Unknown,
}

impl Display for Teletype {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Tty(id) => write!(f, "/dev/tty{id}"),
            Self::TtyS(id) => write!(f, "/dev/ttyS{id}"),
            Self::Pts(id) => write!(f, "/dev/pts/{id}"),
            Self::Unknown => write!(f, "?"),
        }
    }
}

impl TryFrom<String> for Teletype {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&str> for Teletype {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value == "?" {
            return Ok(Self::Unknown);
        }

        Self::try_from(PathBuf::from(value))
    }
}

impl TryFrom<PathBuf> for Teletype {
    type Error = ();

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        // Three case: /dev/pts/* , /dev/ttyS**, /dev/tty**

        let mut iter = value.iter();
        // Case 1

        // Considering this format: **/**/pts/<num>
        if let (Some(_), Some(num)) = (iter.find(|it| *it == "pts"), iter.next()) {
            return num
                .to_str()
                .ok_or(())?
                .parse::<u64>()
                .map_err(|_| ())
                .map(Teletype::Pts);
        }

        // Considering this format: **/**/ttyS** then **/**/tty**
        let path = value.to_str().ok_or(())?;

        let f = |prefix: &str| {
            value
                .iter()
                .next_back()?
                .to_str()?
                .strip_prefix(prefix)?
                .parse::<u64>()
                .ok()
        };

        if path.contains("ttyS") {
            // Case 2
            f("ttyS").ok_or(()).map(Teletype::TtyS)
        } else if path.contains("tty") {
            // Case 3
            f("tty").ok_or(()).map(Teletype::Tty)
        } else {
            Err(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tty_from() {
        assert_eq!(Teletype::try_from("?").unwrap(), Teletype::Unknown);
        assert_eq!(Teletype::try_from("/dev/tty1").unwrap(), Teletype::Tty(1));
        assert_eq!(Teletype::try_from("/dev/tty10").unwrap(), Teletype::Tty(10));
        assert_eq!(Teletype::try_from("/dev/pts/1").unwrap(), Teletype::Pts(1));
        assert_eq!(
            Teletype::try_from("/dev/pts/10").unwrap(),
            Teletype::Pts(10)
        );
        assert_eq!(Teletype::try_from("/dev/ttyS1").unwrap(), Teletype::TtyS(1));
        assert_eq!(
            Teletype::try_from("/dev/ttyS10").unwrap(),
            Teletype::TtyS(10)
        );
        assert_eq!(Teletype::try_from("ttyS10").unwrap(), Teletype::TtyS(10));

        assert!(Teletype::try_from("value").is_err());
        assert!(Teletype::try_from("TtyS10").is_err());
    }

    #[test]
    fn test_terminal_type_display() {
        assert_eq!(Teletype::Pts(10).to_string(), "/dev/pts/10");
        assert_eq!(Teletype::Tty(10).to_string(), "/dev/tty10");
        assert_eq!(Teletype::TtyS(10).to_string(), "/dev/ttyS10");
        assert_eq!(Teletype::Unknown.to_string(), "?");
    }
}
