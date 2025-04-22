// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

/// Fast increment function, operating on ASCII strings.
///
/// Add inc to the string val[start..end]. This operates on ASCII digits, assuming
/// val and inc are well formed.
///
/// Updates `start` if we have a carry, or if inc > start.
///
/// We also assume that there is enough space in val to expand if start needs
/// to be updated.
/// ```
/// use uucore::fast_inc::fast_inc;
///
/// // Start with a buffer containing "0", with one byte of head space
/// let mut val = Vec::from(".0".as_bytes());
/// let mut start = val.len()-1;
/// let end = val.len();
/// let inc = "6".as_bytes();
/// assert_eq!(&val[start..end], "0".as_bytes());
/// fast_inc(val.as_mut(), &mut start, end, inc);
/// assert_eq!(&val[start..end], "6".as_bytes());
/// fast_inc(val.as_mut(), &mut start, end, inc);
/// assert_eq!(&val[start..end], "12".as_bytes());
/// ```
#[inline]
pub fn fast_inc(val: &mut [u8], start: &mut usize, end: usize, inc: &[u8]) {
    // To avoid a lot of casts to signed integers, we make sure to decrement pos
    // as late as possible, so that it does not ever go negative.
    let mut pos = end;
    let mut carry = 0u8;

    // First loop, add all digits of inc into val.
    for inc_pos in (0..inc.len()).rev() {
        // The decrement operation would also panic in debug mode, print a message for developer convenience.
        debug_assert!(
            pos > 0,
            "Buffer overflowed, make sure you allocate val with enough headroom."
        );
        pos -= 1;

        let mut new_val = inc[inc_pos] + carry;
        // Be careful here, only add existing digit of val.
        if pos >= *start {
            new_val += val[pos] - b'0';
        }
        if new_val > b'9' {
            carry = 1;
            new_val -= 10;
        } else {
            carry = 0;
        }
        val[pos] = new_val;
    }

    // Done, now, if we have a carry, add that to the upper digits of val.
    if carry == 0 {
        *start = (*start).min(pos);
        return;
    }

    fast_inc_one(val, start, pos)
}

/// Fast increment by one function, operating on ASCII strings.
///
/// Add 1 to the string val[start..end]. This operates on ASCII digits, assuming
/// val is well formed.
///
/// Updates `start` if we have a carry, or if inc > start.
///
/// We also assume that there is enough space in val to expand if start needs
/// to be updated.
/// ```
/// use uucore::fast_inc::fast_inc_one;
///
/// // Start with a buffer containing "8", with one byte of head space
/// let mut val = Vec::from(".8".as_bytes());
/// let mut start = val.len()-1;
/// let end = val.len();
/// assert_eq!(&val[start..end], "8".as_bytes());
/// fast_inc_one(val.as_mut(), &mut start, end);
/// assert_eq!(&val[start..end], "9".as_bytes());
/// fast_inc_one(val.as_mut(), &mut start, end);
/// assert_eq!(&val[start..end], "10".as_bytes());
/// ```
#[inline]
pub fn fast_inc_one(val: &mut [u8], start: &mut usize, end: usize) {
    let mut pos = end;

    while pos > *start {
        pos -= 1;

        if val[pos] == b'9' {
            // 9+1 = 10. Carry propagating, keep going.
            val[pos] = b'0';
        } else {
            // Carry stopped propagating, return unchanged start.
            val[pos] += 1;
            return;
        }
    }

    // The following decrement operation would also panic in debug mode, print a message for developer convenience.
    debug_assert!(
        *start > 0,
        "Buffer overflowed, make sure you allocate val with enough headroom."
    );
    // The carry propagated so far that a new digit was added.
    val[*start - 1] = b'1';
    *start -= 1;
}

#[cfg(test)]
mod tests {
    use crate::fast_inc::fast_inc;
    use crate::fast_inc::fast_inc_one;

    #[test]
    fn test_fast_inc_simple() {
        let mut val = Vec::from("...0_".as_bytes());
        let mut start: usize = 3;
        let inc = "4".as_bytes();
        fast_inc(val.as_mut(), &mut start, 4, inc);
        assert_eq!(start, 3);
        assert_eq!(val, "...4_".as_bytes());
        fast_inc(val.as_mut(), &mut start, 4, inc);
        assert_eq!(start, 3);
        assert_eq!(val, "...8_".as_bytes());
        fast_inc(val.as_mut(), &mut start, 4, inc);
        assert_eq!(start, 2); // carried 1 more digit
        assert_eq!(val, "..12_".as_bytes());

        let mut val = Vec::from("0_".as_bytes());
        let mut start: usize = 0;
        let inc = "2".as_bytes();
        fast_inc(val.as_mut(), &mut start, 1, inc);
        assert_eq!(start, 0);
        assert_eq!(val, "2_".as_bytes());
        fast_inc(val.as_mut(), &mut start, 1, inc);
        assert_eq!(start, 0);
        assert_eq!(val, "4_".as_bytes());
        fast_inc(val.as_mut(), &mut start, 1, inc);
        assert_eq!(start, 0);
        assert_eq!(val, "6_".as_bytes());
    }

    // Check that we handle increment > val correctly.
    #[test]
    fn test_fast_inc_large_inc() {
        let mut val = Vec::from("...7_".as_bytes());
        let mut start: usize = 3;
        let inc = "543".as_bytes();
        fast_inc(val.as_mut(), &mut start, 4, inc);
        assert_eq!(start, 1); // carried 2 more digits
        assert_eq!(val, ".550_".as_bytes());
        fast_inc(val.as_mut(), &mut start, 4, inc);
        assert_eq!(start, 0); // carried 1 more digit
        assert_eq!(val, "1093_".as_bytes());
    }

    // Check that we handle longer carries
    #[test]
    fn test_fast_inc_carry() {
        let mut val = Vec::from(".999_".as_bytes());
        let mut start: usize = 1;
        let inc = "1".as_bytes();
        fast_inc(val.as_mut(), &mut start, 4, inc);
        assert_eq!(start, 0);
        assert_eq!(val, "1000_".as_bytes());

        let mut val = Vec::from(".999_".as_bytes());
        let mut start: usize = 1;
        let inc = "11".as_bytes();
        fast_inc(val.as_mut(), &mut start, 4, inc);
        assert_eq!(start, 0);
        assert_eq!(val, "1010_".as_bytes());
    }

    #[test]
    fn test_fast_inc_one_simple() {
        let mut val = Vec::from("...8_".as_bytes());
        let mut start: usize = 3;
        fast_inc_one(val.as_mut(), &mut start, 4);
        assert_eq!(start, 3);
        assert_eq!(val, "...9_".as_bytes());
        fast_inc_one(val.as_mut(), &mut start, 4);
        assert_eq!(start, 2); // carried 1 more digit
        assert_eq!(val, "..10_".as_bytes());
        fast_inc_one(val.as_mut(), &mut start, 4);
        assert_eq!(start, 2);
        assert_eq!(val, "..11_".as_bytes());

        let mut val = Vec::from("0_".as_bytes());
        let mut start: usize = 0;
        fast_inc_one(val.as_mut(), &mut start, 1);
        assert_eq!(start, 0);
        assert_eq!(val, "1_".as_bytes());
        fast_inc_one(val.as_mut(), &mut start, 1);
        assert_eq!(start, 0);
        assert_eq!(val, "2_".as_bytes());
        fast_inc_one(val.as_mut(), &mut start, 1);
        assert_eq!(start, 0);
        assert_eq!(val, "3_".as_bytes());
    }
}
