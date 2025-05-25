use std::{cmp::Ordering, sync::OnceLock};

use icu::{
    collator::{Collator, options::CollatorOptions},
    locale::{Locale, locale},
};

fn get_collating_locale() -> &'static Locale {
    static COLLATING_LOCALE: OnceLock<Locale> = OnceLock::new();

    COLLATING_LOCALE.get_or_init(|| {
        // Look at 3 environment variables in the following order
        //
        // 1. LC_ALL
        // 2. LC_COLLATE
        // 3. LANG
        //
        // Or fallback on Posix locale

        let locale_var = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LC_COLLATE"))
            .or_else(|_| std::env::var("LANG"));

        if let Ok(locale) = locale_var {
            if let Some(simple) = locale.split(&['-', '@']).next() {
                let bcp127 = simple.replace("_", "-");
                if let Ok(locale) = Locale::try_from_str(&bcp127) {
                    return locale;
                }
            }
        }
        // Default POSIX locale representing LC_ALL=C
        locale!("en-US-posix")
    })
}

pub fn locale_compare<T: AsRef<str>>(left: T, right: T) -> Ordering {
    let collator = Collator::try_new(get_collating_locale().into(), CollatorOptions::default());

    if let Ok(collator) = collator {
        collator.compare_utf8(left.as_ref().as_bytes(), right.as_ref().as_bytes())
    } else {
        // If no collator can be found from the locale env, use simple string comparison
        left.as_ref().cmp(right.as_ref())
    }
}
