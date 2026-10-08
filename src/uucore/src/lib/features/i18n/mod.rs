// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::sync::OnceLock;

use icu_locale::{Locale, locale};

#[cfg(feature = "i18n-charmap")]
pub mod charmap;
#[cfg(feature = "i18n-collator")]
pub mod collator;
#[cfg(feature = "i18n-datetime")]
pub mod datetime;
#[cfg(feature = "i18n-decimal")]
pub mod decimal;

/// The encoding specified by the locale, if specified
/// Currently only supports ASCII and UTF-8 for the sake of simplicity.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UEncoding {
    Ascii,
    Utf8,
}

// Use "und" (undefined) as the marker for C/POSIX locale
// This ensures real locales like "en-US" won't match
const DEFAULT_LOCALE: Locale = locale!("und");

/// Look at 3 environment variables in the following order
///
/// 1. LC_ALL
/// 2. `locale_name`
/// 3. LANG
///
/// Or fallback on Posix locale, with ASCII encoding.
pub fn get_locale_from_env(locale_name: &str) -> (Locale, UEncoding) {
    locale_from_name(locale_name_from_env(locale_name).as_deref())
}

fn locale_name_from_env(locale_name: &str) -> Option<String> {
    ["LC_ALL", locale_name, "LANG"]
        .iter()
        .find_map(|&key| std::env::var(key).ok().filter(|name| !name.is_empty()))
}

fn locale_from_name(locale_var: Option<&str>) -> (Locale, UEncoding) {
    if let Some(locale_var_str) = locale_var {
        let mut split = locale_var_str.split(&['.', '@']);

        if let Some(simple) = split.next() {
            // Naively convert the locale name to BCP47 tag format.
            //
            // See https://en.wikipedia.org/wiki/IETF_language_tag
            let bcp47 = simple.replace('_', "-");
            let locale = Locale::try_from_str(&bcp47).unwrap_or(DEFAULT_LOCALE);

            // Determine encoding from the locale suffix (e.g. en_US.UTF-8, C.UTF-8).
            let encoding = split
                .next()
                .filter(|enc| {
                    let lower = enc.to_lowercase();
                    lower == "utf-8" || lower == "utf8"
                })
                .map_or(UEncoding::Ascii, |_| UEncoding::Utf8);
            return (locale, encoding);
        }
    }

    get_locale_from_os()
}

/// Returns the preferred user locale.
#[cfg(windows)]
pub fn get_locale_from_os() -> (Locale, UEncoding) {
    use std::mem::MaybeUninit;
    use std::ptr::{null, null_mut};
    use std::str;
    use windows_sys::Win32::Globalization::{
        CP_UTF8, GetThreadPreferredUILanguages, MUI_LANGUAGE_NAME, MUI_MERGE_USER_FALLBACK,
        WideCharToMultiByte,
    };

    /// TODO(MSRV>=1.93): remove in favor of `slice::assume_init_ref`
    ///
    /// # Safety
    ///
    /// Same as the official [`slice::assume_init_ref`](https://doc.rust-lang.org/1.93.0/std/primitive.slice.html#method.assume_init_ref).
    #[allow(clippy::ref_as_ptr)]
    unsafe fn assume_init_ref<T>(s: &[MaybeUninit<T>]) -> &[T] {
        unsafe { &*(s as *const [MaybeUninit<T>] as *const [T]) }
    }

    // Each tag is ~5 chars + NUL. How many languages are realistic? 20?
    // Then that's 120 chars.
    const LEN: usize = 256;

    // NOTE: Worst-case UTF16 -> UTF8 is 3x expansion,
    // but MUI_LANGUAGE_NAME tags are ASCII-only.
    let mut utf16 = [const { MaybeUninit::<u16>::uninit() }; LEN];
    let mut utf8 = [const { MaybeUninit::<u8>::uninit() }; LEN];
    let mut len = utf16.len() as u32;
    let mut num = 0;

    // MUI_MERGE_USER_FALLBACK combines thread -> process -> user preferences. This is preferable
    // over GetUserPreferredUILanguages, since the coreutils may be embedded into a larger app.
    // (It also permits a limited form of unit testing.)
    let ok = unsafe {
        GetThreadPreferredUILanguages(
            MUI_LANGUAGE_NAME | MUI_MERGE_USER_FALLBACK,
            &raw mut num,
            utf16.as_mut_ptr().cast(),
            &raw mut len,
        )
    };
    if ok == 0 || num == 0 {
        return (DEFAULT_LOCALE, UEncoding::Utf8);
    }

    let utf16_len = utf16.len().min(len as usize);
    let utf8_len = unsafe {
        WideCharToMultiByte(
            CP_UTF8,
            0,
            utf16.as_mut_ptr().cast(),
            utf16_len as i32,
            utf8.as_mut_ptr().cast(),
            utf8.len() as i32,
            null(),
            null_mut(),
        )
    };
    if utf8_len == 0 {
        return (DEFAULT_LOCALE, UEncoding::Utf8);
    }

    let utf8 = &utf8[..utf8_len as usize];
    let utf8 = unsafe { assume_init_ref(utf8) };
    let utf8 = unsafe { str::from_utf8_unchecked(utf8) };
    let locale = utf8
        .split_terminator('\0')
        .filter(|lang| !lang.is_empty())
        .find_map(|lang| Locale::try_from_str(lang).ok())
        .unwrap_or(DEFAULT_LOCALE);

    (locale, UEncoding::Utf8)
}

/// Returns the default POSIX locale representing LC_ALL=C.
#[cfg(not(windows))]
pub fn get_locale_from_os() -> (Locale, UEncoding) {
    (DEFAULT_LOCALE, UEncoding::Ascii)
}

/// Get the collating locale from the environment
pub fn get_collating_locale() -> &'static (Locale, UEncoding) {
    static COLLATING_LOCALE: OnceLock<(Locale, UEncoding)> = OnceLock::new();

    COLLATING_LOCALE.get_or_init(|| get_locale_from_env("LC_COLLATE"))
}

/// Get the numeric locale from the environment
pub fn get_numeric_locale() -> &'static (Locale, UEncoding) {
    static NUMERIC_LOCALE: OnceLock<(Locale, UEncoding)> = OnceLock::new();

    NUMERIC_LOCALE.get_or_init(|| get_locale_from_env("LC_NUMERIC"))
}

/// Return the encoding deduced from the locale environment variable.
pub fn get_locale_encoding() -> UEncoding {
    get_collating_locale().1
}

/// Return the character-type encoding (`LC_CTYPE`) selected by the environment.
/// Bare locale names are resolved through native locale data where available.
pub fn get_ctype_encoding() -> UEncoding {
    static CTYPE_ENCODING: OnceLock<UEncoding> = OnceLock::new();

    *CTYPE_ENCODING.get_or_init(|| {
        let name = locale_name_from_env("LC_CTYPE");
        if let Some(name) = name.as_deref()
            && !name.contains('.')
            && !matches!(name, "C" | "POSIX")
            && let Some(encoding) = encoding_for_bare_locale(name)
        {
            return encoding;
        }
        locale_from_name(name.as_deref()).1
    })
}

fn encoding_for_bare_locale(name: &str) -> Option<UEncoding> {
    #[cfg(any(
        target_os = "linux",
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "illumos",
        target_os = "solaris",
        target_os = "aix",
        target_os = "hurd"
    ))]
    {
        use std::ffi::{CStr, CString};
        // libc has no nl_langinfo_l binding on Apple and OpenBSD targets.
        unsafe extern "C" {
            fn nl_langinfo_l(item: libc::nl_item, locale: libc::locale_t) -> *mut libc::c_char;
        }
        let name = CString::new(name).ok()?;
        // SAFETY: name is a NUL-terminated string. A null base creates an owned
        // locale, without changing the process-wide locale.
        let locale =
            unsafe { libc::newlocale(libc::LC_CTYPE_MASK, name.as_ptr(), std::ptr::null_mut()) };
        if locale.is_null() {
            return None;
        }
        // SAFETY: locale is owned and live while its CODESET string is read.
        // It is freed only after the string has been inspected.
        unsafe {
            let codeset = nl_langinfo_l(libc::CODESET, locale);
            let encoding = if codeset.is_null() {
                None
            } else if matches!(CStr::from_ptr(codeset).to_bytes(), b"UTF-8" | b"UTF8") {
                Some(UEncoding::Utf8)
            } else {
                Some(UEncoding::Ascii)
            };
            libc::freelocale(locale);
            encoding
        }
    }
    #[cfg(not(any(
        target_os = "linux",
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "illumos",
        target_os = "solaris",
        target_os = "aix",
        target_os = "hurd"
    )))]
    {
        let _ = name;
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn locale_suffixes_keep_their_encoding() {
        for (name, expected) in [
            ("C.UTF-8", super::UEncoding::Utf8),
            ("en_IN.UTF8", super::UEncoding::Utf8),
            ("en_US.ISO-8859-1", super::UEncoding::Ascii),
            ("POSIX", super::UEncoding::Ascii),
        ] {
            assert_eq!(super::locale_from_name(Some(name)).1, expected, "{name}");
        }
    }

    #[test]
    fn locale_names_with_nul_have_no_native_encoding() {
        assert_eq!(super::encoding_for_bare_locale("en_IN\0"), None);
    }

    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    #[test]
    fn bare_locales_use_their_installed_encoding() {
        assert_eq!(super::encoding_for_bare_locale("nonexistent_LOCALE"), None);
        assert_eq!(
            super::encoding_for_bare_locale("C"),
            Some(super::UEncoding::Ascii)
        );
        assert_eq!(
            super::encoding_for_bare_locale("POSIX"),
            Some(super::UEncoding::Ascii)
        );
        for (name, expected) in [
            ("en_IN", super::UEncoding::Utf8),
            ("en_US", super::UEncoding::Ascii),
        ] {
            if let Some(encoding) = super::encoding_for_bare_locale(name) {
                assert_eq!(encoding, expected, "{name}");
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_get_locale_from_os() {
        use icu_locale::locale;
        use std::ptr::null;
        use windows_sys::Win32::Globalization::{MUI_LANGUAGE_NAME, SetThreadPreferredUILanguages};
        use windows_sys::w;

        // Unfortunately it's not possible to test if multiple languages parse properly.
        // `Locale::try_from_str` succeeds on the first valid tag and
        // `SetThreadPreferredUILanguages` does not allow setting invalid tags.
        unsafe {
            const LANGS: *const u16 = w!("fr-FR\0");
            let mut num = 0;
            SetThreadPreferredUILanguages(MUI_LANGUAGE_NAME, LANGS, &raw mut num);
            assert_eq!(num, 1);
        }

        let (locale, encoding) = super::get_locale_from_os();
        assert_eq!(encoding, super::UEncoding::Utf8);
        assert_eq!(locale, locale!("fr-FR"));

        unsafe {
            let mut num = 0;
            SetThreadPreferredUILanguages(MUI_LANGUAGE_NAME, null(), &raw mut num);
            assert_eq!(num, 0);
        }
    }
}
