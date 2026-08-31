// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore langneg Hans Hant gettext

//! Pairing the locale a system asks for with the translations that exist.
//!
//! The two are named in different worlds. A POSIX environment asks with a
//! region — `zh_CN`, `de_DE`, `es_MX` — while a translation is filed under
//! whatever distinction its translators needed: a language (`de.ftl`), a
//! region (`pt-BR.ftl`) or a script (`zh-Hans.ftl`). Taking the request for a
//! file name therefore finds only the translations whose name happens to
//! coincide with a locale's, which is why `LANG=zh_CN.UTF-8` used to read no
//! Chinese at all ([#12305]).
//!
//! Matching the two is language negotiation, specified by [RFC 4647] and
//! [UTS #35] and implemented by `fluent-langneg`. This module is the seam
//! between it, the environment and the way this crate files its translations.
//!
//! [#12305]: https://github.com/uutils/coreutils/issues/12305
//! [RFC 4647]: https://www.rfc-editor.org/rfc/rfc4647
//! [UTS #35]: https://www.unicode.org/reports/tr35/#LanguageMatching

use std::fs;
use std::path::Path;

use fluent_langneg::{NegotiationStrategy, negotiate_languages};
use unic_langid::LanguageIdentifier;

use super::locale::DEFAULT_LOCALE;

/// The locales the environment asks for, in the order it prefers them.
///
/// `var` looks up an environment variable. Taking it as an argument rather than
/// reading the environment here keeps this testable: writing to `std::env` is
/// unsound once a second thread exists, and the test binary is threaded.
///
/// The variables and the order are gettext's, so that these utilities answer to
/// the same environment the GNU ones do:
///
/// * `LC_ALL`, `LC_MESSAGES` and `LANG` name the locale; the first one set
///   wins, as POSIX specifies.
/// * `LANGUAGE` then replaces it with a colon-separated list of locales to try
///   in turn — unless the locale named above was `C` or `POSIX`, since a list
///   of languages should not override a request for no translation at all.
///
/// Values arrive in POSIX form, so the encoding and the modifier — the `.UTF-8`
/// of `zh_CN.UTF-8`, the `@euro` of `de_DE@euro` — are dropped; neither says
/// anything about which language to speak. A value that is no language tag at
/// all is skipped rather than discarding the ones around it.
pub fn requested_locales(var: impl Fn(&str) -> Option<String>) -> Vec<LanguageIdentifier> {
    let read = |name: &str| var(name).filter(|value| !value.is_empty());

    // An unset locale is the C locale, which asks for no translation.
    let Some(value) = ["LC_ALL", "LC_MESSAGES", "LANG"].into_iter().find_map(read) else {
        return vec![default_locale()];
    };
    let named = language_part(&value);
    if named == "C" || named == "POSIX" {
        return vec![default_locale()];
    }

    let language = read("LANGUAGE");
    let mut requested = Vec::new();
    for entry in language.as_deref().unwrap_or(named).split(':') {
        if let Ok(locale) = language_part(entry).parse::<LanguageIdentifier>()
            && !requested.contains(&locale)
        {
            requested.push(locale);
        }
    }
    requested
}

/// The locales `dir` holds translations for.
///
/// Sorted, so that which of two equally good translations is read does not
/// depend on the order the directory happens to be read in. An unreadable
/// directory simply holds no translations.
pub fn available_locales(dir: &Path) -> Vec<LanguageIdentifier> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut locales: Vec<LanguageIdentifier> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().is_none_or(|extension| extension != "ftl") {
                return None;
            }
            path.file_stem()?.to_str()?.parse().ok()
        })
        .collect();
    locales.sort();
    locales
}

/// The translations among `available` that serve `requested`, best first.
///
/// A locale nothing serves negotiates to nothing; English is not appended here,
/// because the caller loads it by a path of its own — it is embedded in the
/// binary, and the fallback behind whatever this returns.
///
/// Filtering, rather than picking a single winner, is what lets a locale reach a
/// translation it only partly agrees with: `es_MX` reads `es-ES.ftl` because the
/// region is the only thing they differ in, while `zh_TW` never reads
/// `zh-Hans.ftl` because their scripts genuinely disagree.
pub fn negotiate(
    requested: &[LanguageIdentifier],
    available: &[LanguageIdentifier],
) -> Vec<LanguageIdentifier> {
    negotiate_languages(requested, available, None, NegotiationStrategy::Filtering)
        .into_iter()
        .cloned()
        .collect()
}

/// [`DEFAULT_LOCALE`] parsed.
pub fn default_locale() -> LanguageIdentifier {
    DEFAULT_LOCALE
        .parse()
        .expect("the default locale is a literal language tag")
}

/// `value` up to the encoding or the modifier a POSIX locale name may carry.
fn language_part(value: &str) -> &str {
    let value = value.trim();
    &value[..value.find(['.', '@']).unwrap_or(value.len())]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    /// The locales `ls` is translated into, i.e. a realistic set of names for
    /// negotiation to aim at: two of them name a script, most a bare language,
    /// a few a region.
    fn ls_locales() -> Vec<LanguageIdentifier> {
        "ar ast ca cs da de en-US eo es-ES fi fr-FR he hu id it ja kab ko nb-NO ne nl pl pt-BR \
         pt ru sv tr uk vi zh-Hans zh-Hant"
            .split_whitespace()
            .map(|tag| tag.parse().unwrap())
            .collect()
    }

    fn from_lang(lang: &str) -> Vec<LanguageIdentifier> {
        requested_locales(|name| (name == "LANG").then(|| lang.to_owned()))
    }

    /// The translation a `LANG` value ends up reading out of [`ls_locales`].
    fn best_for(lang: &str) -> String {
        negotiate(&from_lang(lang), &ls_locales())
            .first()
            .expect("a translation serving this locale is on offer")
            .to_string()
    }

    /// Every locale reaches the translation meant for it, whichever kind of
    /// name that translation happens to be filed under.
    ///
    /// Before negotiation these were matched as literal file names, so of the
    /// locales below only `en_US`, `es_ES`, `pt_BR`, `nb_NO`, `zh_Hans` and
    /// `zh_Hant` read anything but English.
    ///
    /// <https://github.com/uutils/coreutils/issues/12305>
    #[test]
    fn every_locale_reaches_the_translation_meant_for_it() {
        // A region reaching the script it is written in, which is the whole of
        // the Chinese case: none of these name their script, and Simplified and
        // Traditional must not be confused for one another.
        assert_eq!(best_for("zh_CN.UTF-8"), "zh-Hans");
        assert_eq!(best_for("zh_SG.UTF-8"), "zh-Hans");
        assert_eq!(best_for("zh_TW.UTF-8"), "zh-Hant");
        assert_eq!(best_for("zh_HK.UTF-8"), "zh-Hant");
        assert_eq!(best_for("zh_MO.UTF-8"), "zh-Hant");
        // A bare language takes the script it is most commonly written in, and
        // one naming its script reads it directly.
        assert_eq!(best_for("zh"), "zh-Hans");
        assert_eq!(best_for("zh_Hant"), "zh-Hant");

        // A region reaching a translation filed under the bare language.
        for lang in ["de_DE.UTF-8", "de_AT", "ja_JP.UTF-8", "pt_PT", "kab_DZ"] {
            let expected = lang.split(['_', '.']).next().unwrap();
            assert_eq!(best_for(lang), expected, "for LANG={lang}");
        }

        // A region reaching a translation filed under a different region: the
        // region is the only thing the two disagree about.
        assert_eq!(best_for("es_MX.UTF-8"), "es-ES");
        assert_eq!(best_for("fr_CA"), "fr-FR");
        assert_eq!(best_for("en_GB.UTF-8"), "en-US");
        // And a bare language reaching one filed under a region.
        assert_eq!(best_for("nb"), "nb-NO");

        // A language with neither script nor region on either side.
        assert_eq!(best_for("eo"), "eo");

        // A modifier names a currency or an orthography, never a language.
        assert_eq!(best_for("de_DE@euro"), "de");
        assert_eq!(best_for("ca_ES@valencia"), "ca");
    }

    /// A locale nothing is translated into negotiates to nothing, rather than
    /// to English: the caller loads English by a path of its own, and would
    /// otherwise read and parse the same file twice.
    #[test]
    fn an_untranslated_locale_negotiates_to_nothing() {
        // Tamil is among the languages nobody has translated.
        assert!(negotiate(&from_lang("ta_IN"), &ls_locales()).is_empty());
        assert!(negotiate(&from_lang("fr_FR.UTF-8"), &[]).is_empty());
    }

    /// A locale reads a translation it disagrees with about the script no more
    /// than it reads one in another language: Taiwan does not read Simplified
    /// Chinese, however close the two look as language tags.
    #[test]
    fn a_disagreeing_script_is_not_a_match() {
        let simplified: [LanguageIdentifier; 1] = ["zh-Hans".parse().unwrap()];
        assert!(negotiate(&from_lang("zh_TW.UTF-8"), &simplified).is_empty());
        assert_eq!(negotiate(&from_lang("zh_CN"), &simplified), simplified);
    }

    /// Every translation that serves the locale is offered, closest first, so
    /// that one which turns out to be unreadable is not the end of it.
    #[test]
    fn the_closest_translation_comes_first() {
        let names = |locales: Vec<LanguageIdentifier>| {
            locales
                .iter()
                .map(LanguageIdentifier::to_string)
                .collect::<Vec<_>>()
        };
        let available = ls_locales();
        assert_eq!(
            names(negotiate(&from_lang("pt_BR"), &available)),
            ["pt-BR", "pt"]
        );
        assert_eq!(
            names(negotiate(&from_lang("pt_PT"), &available)),
            ["pt", "pt-BR"]
        );
    }

    #[test]
    fn a_directory_offers_the_ftl_files_it_holds() {
        let dir = TempDir::new().unwrap();
        for name in ["en-US.ftl", "zh-Hans.ftl", "de.ftl", "README.md", "notes"] {
            fs::write(dir.path().join(name), "").unwrap();
        }

        let held: Vec<String> = available_locales(dir.path())
            .iter()
            .map(LanguageIdentifier::to_string)
            .collect();
        assert_eq!(held, ["de", "en-US", "zh-Hans"]);
        assert!(available_locales(&dir.path().join("nonexistent")).is_empty());
    }

    /// The variables POSIX and gettext define, in the order they define them.
    #[test]
    fn the_environment_is_read_the_way_gettext_reads_it() {
        let requested = |vars: &[(&str, &str)]| {
            let vars: HashMap<String, String> = vars
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect();
            requested_locales(|name| vars.get(name).cloned())
                .iter()
                .map(LanguageIdentifier::to_string)
                .collect::<Vec<_>>()
        };

        // LC_ALL over LC_MESSAGES over LANG.
        assert_eq!(requested(&[("LANG", "fr_FR.UTF-8")]), ["fr-FR"]);
        assert_eq!(
            requested(&[("LC_MESSAGES", "de_DE.UTF-8"), ("LANG", "fr_FR.UTF-8")]),
            ["de-DE"]
        );
        assert_eq!(
            requested(&[
                ("LC_ALL", "ja_JP.UTF-8"),
                ("LC_MESSAGES", "de_DE.UTF-8"),
                ("LANG", "fr_FR.UTF-8"),
            ]),
            ["ja-JP"]
        );

        // LANGUAGE lists what to try, in its own order, and an empty value is
        // not a value.
        assert_eq!(
            requested(&[("LANGUAGE", "zh_TW:zh"), ("LANG", "fr_FR.UTF-8")]),
            ["zh-TW", "zh"]
        );
        assert_eq!(
            requested(&[("LANGUAGE", ""), ("LANG", "fr_FR.UTF-8")]),
            ["fr-FR"]
        );

        // The C locale asks for no translation, and no language list overrides
        // that. Neither does an unset one.
        assert_eq!(requested(&[("LANG", "C.UTF-8")]), [DEFAULT_LOCALE]);
        assert_eq!(
            requested(&[("LANGUAGE", "de"), ("LC_ALL", "POSIX")]),
            [DEFAULT_LOCALE]
        );
        assert_eq!(requested(&[]), [DEFAULT_LOCALE]);

        // A value that is no language tag is dropped, and the rest of the list
        // survives it. A locale repeated across the list is asked for once.
        assert!(requested(&[("LANG", "@@@@")]).is_empty());
        assert_eq!(
            requested(&[("LANGUAGE", "@@@@:de"), ("LANG", "fr")]),
            ["de"]
        );
        assert_eq!(
            requested(&[("LANGUAGE", "de:de_DE:de"), ("LANG", "fr")]),
            ["de", "de-DE"]
        );
    }
}
