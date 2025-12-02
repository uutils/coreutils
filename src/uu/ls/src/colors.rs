// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use super::PathData;
use lscolors::{Colorable, Indicator, LsColors, Style};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::Metadata;

// The lscolors crate does not keep the original SGR token order or leading zeros.
// Keep the raw LS_COLORS entries so we can emit sequences that match GNU ls.
fn raw_ls_colors_map() -> HashMap<Indicator, String> {
    // Copy the same defaults that lscolors uses internally.
    const DEFAULT_LS_COLORS: &str = "rs=0:lc=\u{1b}[:rc=m:cl=\u{1b}[K:ex=01;32:sg=30;43:su=37;41:di=01;34:st=37;44:ow=34;42:tw=30;42:ln=01;36:bd=01;33:cd=01;33:do=01;35:pi=33:so=01;35:";

    let mut map = HashMap::new();

    let mut apply = |spec: &str| {
        for entry in spec.split(':') {
            if let Some((key, value)) = entry.split_once('=') {
                if let Some(indicator) = Indicator::from(key) {
                    map.insert(indicator, value.to_string());
                }
            }
        }
    };

    apply(DEFAULT_LS_COLORS);
    if let Ok(env_spec) = std::env::var("LS_COLORS") {
        if !env_spec.is_empty() {
            apply(&env_spec);
        }
    }

    map
}

/// We need this struct to be able to store the previous style.
/// This because we need to check the previous value in case we don't need
/// the reset
pub(crate) struct StyleManager<'a> {
    /// last style that is applied, if `None` that means reset is applied.
    pub(crate) current_style: Option<Style>,
    /// `true` if the initial reset is applied
    pub(crate) initial_reset_is_done: bool,
    pub(crate) colors: &'a LsColors,
    pub(crate) symlink_as_target: bool,
    pub(crate) orphan_color_explicit: bool,
    pub(crate) raw_ls_colors: HashMap<Indicator, String>,
}

impl<'a> StyleManager<'a> {
    pub(crate) fn new(
        colors: &'a LsColors,
        symlink_as_target: bool,
        orphan_color_explicit: bool,
    ) -> Self {
        Self {
            initial_reset_is_done: false,
            current_style: None,
            colors,
            symlink_as_target,
            orphan_color_explicit,
            raw_ls_colors: raw_ls_colors_map(),
        }
    }

    pub(crate) fn apply_style(
        &mut self,
        new_style: Option<&Style>,
        name: OsString,
        wrap: bool,
        indicator_hint: Option<Indicator>,
    ) -> OsString {
        let mut style_code = String::new();
        let mut force_suffix_reset: bool = false;

        // if reset is done we need to apply normal style before applying new style
        if self.is_reset() {
            if let Some(norm_sty) = self.get_normal_style().copied() {
                style_code.push_str(&self.get_style_code(&norm_sty, None));
            }
        }

        if let Some(new_style) = new_style {
            // we only need to apply a new style if it's not the same as the current
            // style for example if normal is the current style and a file with
            // normal style is to be printed we could skip printing new color
            // codes
            if !self.is_current_style(new_style) {
                style_code.push_str(self.reset(!self.initial_reset_is_done));
                style_code.push_str(&self.get_style_code(new_style, indicator_hint));
            }
        }
        // if new style is None and current style is Normal we should reset it
        else if matches!(self.get_normal_style().copied(), Some(norm_style) if self.is_current_style(&norm_style))
        {
            style_code.push_str(self.reset(false));
            // even though this is an unnecessary reset for gnu compatibility we allow it here
            force_suffix_reset = true;
        }

        // we need this clear to eol code in some terminals, for instance if the
        // text is in the last row of the terminal meaning the terminal need to
        // scroll up in order to print new text in this situation if the clear
        // to eol code is not present the background of the text would stretch
        // till the end of line
        let clear_to_eol = if wrap { "\x1b[K" } else { "" };

        let mut ret: OsString = style_code.into();
        ret.push(name);
        ret.push(self.reset(force_suffix_reset));
        ret.push(clear_to_eol);
        ret
    }

    /// Resets the current style and returns the default ANSI reset code to
    /// reset all text formatting attributes. If `force` is true, the reset is
    /// done even if the reset has been applied before.
    pub(crate) fn reset(&mut self, force: bool) -> &'static str {
        // todo:
        // We need to use style from `Indicator::Reset` but as of now ls colors
        // uses a fallback mechanism and because of that if `Indicator::Reset`
        // is not specified it would fallback to `Indicator::Normal` which seems
        // to be non compatible with gnu
        if self.current_style.is_some() || force {
            self.initial_reset_is_done = true;
            self.current_style = None;
            return "\x1b[0m";
        }
        ""
    }

    pub(crate) fn get_style_code(
        &mut self,
        new_style: &Style,
        indicator_hint: Option<Indicator>,
    ) -> String {
        if let Some(ind) = indicator_hint {
            if let Some(raw) = self.raw_ls_colors.get(&ind) {
                self.current_style = Some(*new_style);
                return format!("\x1b[{raw}m");
            }
        }

        self.current_style = Some(*new_style);
        let mut nu_a_style = new_style.to_nu_ansi_term_style();
        nu_a_style.prefix_with_reset = false;
        let mut ret = nu_a_style.paint("").to_string();
        // remove the suffix reset
        ret.truncate(ret.len() - 4);
        ret
    }

    pub(crate) fn is_current_style(&self, new_style: &Style) -> bool {
        matches!(&self.current_style, Some(style) if style == new_style)
    }

    pub(crate) fn is_reset(&self) -> bool {
        self.current_style.is_none()
    }

    pub(crate) fn get_normal_style(&self) -> Option<&Style> {
        self.colors.style_for_indicator(Indicator::Normal)
    }
    pub(crate) fn apply_normal(&mut self) -> String {
        if let Some(sty) = self.get_normal_style().copied() {
            return self.get_style_code(&sty, None);
        }
        String::new()
    }

    pub(crate) fn apply_style_based_on_metadata(
        &mut self,
        path: &PathData,
        md_option: Option<&Metadata>,
        name: OsString,
        wrap: bool,
        indicator_hint: Option<Indicator>,
    ) -> OsString {
        let style = self
            .colors
            .style_for_path_with_metadata(&path.p_buf, md_option);
        self.apply_style(style, name, wrap, indicator_hint)
    }

    pub(crate) fn apply_style_based_on_colorable<T: Colorable>(
        &mut self,
        path: &T,
        name: OsString,
        wrap: bool,
        indicator_hint: Option<Indicator>,
    ) -> OsString {
        let style = self.colors.style_for(path);
        self.apply_style(style, name, wrap, indicator_hint)
    }
}

/// Colors the provided name based on the style determined for the given path
pub(crate) fn color_name(
    name: OsString,
    path: &PathData,
    style_manager: &mut StyleManager,
    target_symlink: Option<&PathData>,
    wrap: bool,
) -> OsString {
    // For symlinks we want to honor the exact LS_COLORS sequence, so detect them explicitly.
    let symlink_indicator = if style_manager.symlink_as_target {
        None
    } else if path.file_type().is_some_and(|ft| ft.is_symlink()) {
        let orphan = style_manager
            .colors
            .has_explicit_style_for(Indicator::OrphanedSymbolicLink)
            && !path.path().exists();
        if orphan {
            Some(Indicator::OrphanedSymbolicLink)
        } else {
            Some(Indicator::SymbolicLink)
        }
    } else {
        None
    };

    // If we failed to obtain any metadata and don't even know the file type,
    // treat the entry as missing so that `mi=` from LS_COLORS is honored
    // (e.g. for dangling symlink targets).
    if path.file_type().is_none() && path.metadata().is_none() {
        if let Some(style) = style_manager
            .colors
            .style_for_indicator(Indicator::MissingFile)
        {
            return style_manager.apply_style(
                Some(style),
                name,
                wrap,
                Some(Indicator::MissingFile),
            );
        }
    }

    if style_manager.symlink_as_target && path.file_type().is_some_and(|ft| ft.is_symlink()) {
        let target_metadata = target_symlink
            .and_then(|p| p.metadata().cloned())
            .or_else(|| {
                path.path().read_link().ok().and_then(|target| {
                    let mut absolute = target.clone();
                    if target.is_relative() {
                        if let Some(parent) = path.path().parent() {
                            absolute = parent.join(absolute);
                        }
                    }
                    std::fs::metadata(&absolute).ok()
                })
            });

        let orphan_explicit = style_manager
            .colors
            .has_explicit_style_for(Indicator::OrphanedSymbolicLink)
            || style_manager.orphan_color_explicit;

        let style = if let Some(md) = target_metadata.as_ref() {
            style_manager
                .colors
                .style_for_path_with_metadata(&path.p_buf, Some(md))
        } else if orphan_explicit {
            style_manager
                .colors
                .style_for_indicator(Indicator::OrphanedSymbolicLink)
        } else {
            // When `ln=target` is set but no orphan style is provided
            // (e.g. `or=` to disable coloring), GNU ls leaves dangling
            // symlinks uncolored.  Avoid falling back to the default
            // symlink color in that case.
            None
        };

        // If the orphan style was explicitly mentioned but yields no color
        // (e.g. `or=`), still wrap the name in reset codes for GNU parity.
        if style.is_none() && orphan_explicit {
            let mut ret: OsString = style_manager.reset(true).into();
            ret.push("\x1b[m");
            ret.push(name);
            ret.push(style_manager.reset(true));
            if wrap {
                ret.push("\x1b[K");
            }
            return ret;
        }

        return style_manager.apply_style(style, name, wrap, symlink_indicator);
    }

    // Check if the file has capabilities
    #[cfg(all(unix, not(any(target_os = "android", target_os = "macos"))))]
    {
        // Skip checking capabilities if LS_COLORS=ca=:
        let capabilities = style_manager
            .colors
            .style_for_indicator(Indicator::Capabilities);

        let has_capabilities = if capabilities.is_none() {
            false
        } else {
            uucore::fsxattr::has_acl(path.p_buf.as_path())
        };

        // If the file has capabilities, use a specific style for `ca` (capabilities)
        if has_capabilities {
            return style_manager.apply_style(capabilities, name, wrap, None);
        }
    }

    if !path.must_dereference {
        // Avoid triggering metadata/stat when d_type is available; rely solely on the path and d_type.
        let style = style_manager
            .colors
            .style_for_path_with_metadata(&path.p_buf, None);
        return style_manager.apply_style(style, name, wrap, symlink_indicator);
    }

    if let Some(target) = target_symlink {
        // use the optional target_symlink
        // Use fn symlink_metadata directly instead of get_metadata() here because ls
        // should not exit with an err, if we are unable to obtain the target_metadata
        style_manager.apply_style_based_on_colorable(target, name, wrap, None)
    } else {
        let md_option: Option<Metadata> = path
            .metadata()
            .cloned()
            .or_else(|| path.p_buf.symlink_metadata().ok());

        // If dereferencing failed (no metadata) but we know it's a symlink, fall back to
        // styling the link itself rather than treating it as missing.
        if md_option.is_none() && path.file_type().is_some_and(|ft| ft.is_symlink()) {
            return style_manager.apply_style_based_on_colorable(
                path,
                name,
                wrap,
                Some(Indicator::SymbolicLink),
            );
        }

        style_manager.apply_style_based_on_metadata(
            path,
            md_option.as_ref(),
            name,
            wrap,
            symlink_indicator,
        )
    }
}
