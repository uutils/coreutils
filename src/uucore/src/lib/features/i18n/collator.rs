use std::{cmp::Ordering, sync::OnceLock};

use icu_collator::{self, CollatorBorrowed};

use crate::i18n::{DEFAULT_LOCALE, get_collating_locale};

pub use icu_collator::options::{
    AlternateHandling, CaseLevel, CollatorOptions, MaxVariable, Strength,
};

static COLLATOR: OnceLock<CollatorBorrowed> = OnceLock::new();

/// Will initialize the collator if not already initialized.
/// returns `true` if initialization happened
pub fn try_init_collator(opts: CollatorOptions) -> bool {
    COLLATOR
        .set(CollatorBorrowed::try_new(get_collating_locale().0.clone().into(), opts).unwrap())
        .is_ok()
}

/// Will initialize the collator and panic if already initialized.
pub fn init_collator(opts: CollatorOptions) {
    COLLATOR
        .set(CollatorBorrowed::try_new(get_collating_locale().0.clone().into(), opts).unwrap())
        .expect("Collator already initialized");
}

/// Compare both strings with regard to the current locale.
pub fn locale_cmp(left: &[u8], right: &[u8]) -> Ordering {
    // If the detected locale is 'C', just do byte-wise comparison
    if get_collating_locale().0 == DEFAULT_LOCALE {
        left.cmp(right)
    } else {
        COLLATOR
            .get()
            .expect("Collator was not initialized")
            .compare_utf8(left, right)
    }
}
