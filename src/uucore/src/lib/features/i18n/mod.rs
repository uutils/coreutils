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
    let locale_var = ["LC_ALL", locale_name, "LANG"]
        .iter()
        .find_map(|&key| std::env::var(key).ok().filter(|l| !l.is_empty()));

    locale_var.map_or_else(get_locale_from_os, |value| parse_posix_locale(&value))
}

/// Parse `language[_territory][.codeset][@modifier]` from a POSIX locale name.
///
/// C and POSIX use the default collation. Only a codeset following `.` can
/// select UTF-8; a modifier following `@` is never an encoding.
/// Unrecognized locale names use the default locale; unrecognized codesets use ASCII.
pub fn parse_posix_locale(value: &str) -> (Locale, UEncoding) {
    let name_and_codeset = value.split_once('@').map_or(value, |(name, _)| name);
    let (name, codeset) = name_and_codeset
        .split_once('.')
        .unwrap_or((name_and_codeset, ""));
    let locale = if matches!(name, "C" | "POSIX") {
        DEFAULT_LOCALE
    } else {
        Locale::try_from_str(&name.replace('_', "-")).unwrap_or(DEFAULT_LOCALE)
    };
    let encoding = if codeset.eq_ignore_ascii_case("utf-8") || codeset.eq_ignore_ascii_case("utf8")
    {
        UEncoding::Utf8
    } else {
        UEncoding::Ascii
    };
    (locale, encoding)
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

/// Return the character-type encoding (`LC_CTYPE`) deduced from the environment.
pub fn get_ctype_encoding() -> UEncoding {
    static CTYPE_ENCODING: OnceLock<UEncoding> = OnceLock::new();

    *CTYPE_ENCODING.get_or_init(|| get_locale_from_env("LC_CTYPE").1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_do_not_select_an_encoding() {
        for value in ["en_US@utf-8", "en_US@utf-8.dict", "en_US@latin.UTF-8"] {
            assert_eq!(
                parse_posix_locale(value),
                (locale!("en-US"), UEncoding::Ascii)
            );
        }
        for value in ["en_US.UTF-8@latin", "en_US.utf8", "en_US.UTF8@utf-8.dict"] {
            assert_eq!(
                parse_posix_locale(value),
                (locale!("en-US"), UEncoding::Utf8)
            );
        }
    }

    #[test]
    fn posix_and_c_names_use_default_collation() {
        for value in ["C", "POSIX", "C@modifier", "POSIX@modifier"] {
            assert_eq!(
                parse_posix_locale(value),
                (DEFAULT_LOCALE, UEncoding::Ascii)
            );
        }
        for value in ["C.UTF-8", "POSIX.UTF8@modifier"] {
            assert_eq!(parse_posix_locale(value), (DEFAULT_LOCALE, UEncoding::Utf8));
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

        let (locale, encoding) = get_locale_from_os();
        assert_eq!(encoding, UEncoding::Utf8);
        assert_eq!(locale, locale!("fr-FR"));

        unsafe {
            let mut num = 0;
            SetThreadPreferredUILanguages(MUI_LANGUAGE_NAME, null(), &raw mut num);
            assert_eq!(num, 0);
        }
    }
}
