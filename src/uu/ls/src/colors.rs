// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use super::PathData;
use lscolors::{Indicator, LsColors, Style};
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs::{self, Metadata};
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt};

/// We need this struct to be able to store the previous style.
/// This because we need to check the previous value in case we don't need
/// the reset
pub(crate) struct StyleManager<'a> {
    /// last style that is applied, if `None` that means reset is applied.
    pub(crate) current_style: Option<Style>,
    /// `true` if the initial reset is applied
    pub(crate) initial_reset_is_done: bool,
    pub(crate) colors: &'a LsColors,
    /// raw indicator codes as specified in LS_COLORS (if available)
    indicator_codes: HashMap<Indicator, String>,
    /// whether ln=target is active
    ln_color_from_target: bool,
}

impl<'a> StyleManager<'a> {
    pub(crate) fn new(colors: &'a LsColors) -> Self {
        let (indicator_codes, ln_color_from_target) = parse_indicator_codes();
        Self {
            initial_reset_is_done: false,
            current_style: None,
            colors,
            indicator_codes,
            ln_color_from_target,
        }
    }

    pub(crate) fn apply_style(
        &mut self,
        new_style: Option<&Style>,
        path: Option<&PathData>,
        name: OsString,
        wrap: bool,
    ) -> OsString {
        let mut style_code = String::new();
        let mut force_suffix_reset: bool = false;
        let mut applied_raw_code = false;

        if self.is_reset() {
            if let Some(norm_sty) = self.get_normal_style().copied() {
                style_code.push_str(&self.get_style_code(&norm_sty));
            }
        }

        if let Some(path) = path {
            if let Some(indicator) = self.indicator_for_raw_code(path) {
                let should_skip = indicator == Indicator::SymbolicLink
                    && self.ln_color_from_target
                    && path.path().exists();

                if !should_skip {
                    if let Some(raw) = self.indicator_codes.get(&indicator).cloned() {
                        if raw.is_empty() {
                            return self.apply_empty_style(name, wrap);
                        }
                        style_code.push_str(self.reset(!self.initial_reset_is_done));
                        style_code.push_str("\x1b[");
                        style_code.push_str(&raw);
                        style_code.push('m');
                        applied_raw_code = true;
                        self.current_style = None;
                        force_suffix_reset = true;
                    }
                }
            }
        }

        if !applied_raw_code {
            if let Some(new_style) = new_style {
                // we only need to apply a new style if it's not the same as the current
                // style for example if normal is the current style and a file with
                // normal style is to be printed we could skip printing new color
                // codes
                if !self.is_current_style(new_style) {
                    style_code.push_str(self.reset(!self.initial_reset_is_done));
                    style_code.push_str(&self.get_style_code(new_style));
                }
            }
            // if new style is None and current style is Normal we should reset it
            else if matches!(self.get_normal_style().copied(), Some(norm_style) if self.is_current_style(&norm_style))
            {
                style_code.push_str(self.reset(false));
                // even though this is an unnecessary reset for gnu compatibility we allow it here
                force_suffix_reset = true;
            }
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

    pub(crate) fn get_style_code(&mut self, new_style: &Style) -> String {
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
            return self.get_style_code(&sty);
        }
        String::new()
    }

    pub(crate) fn apply_style_based_on_metadata(
        &mut self,
        path: &PathData,
        md_option: Option<&Metadata>,
        name: OsString,
        wrap: bool,
    ) -> OsString {
        let style = self
            .colors
            .style_for_path_with_metadata(&path.p_buf, md_option);
        self.apply_style(style, Some(path), name, wrap)
    }

    pub(crate) fn apply_style_for_path(
        &mut self,
        path: &PathData,
        name: OsString,
        wrap: bool,
    ) -> OsString {
        let style = self.colors.style_for(path);
        self.apply_style(style, Some(path), name, wrap)
    }

    pub(crate) fn apply_indicator_style(
        &mut self,
        indicator: Indicator,
        name: OsString,
        wrap: bool,
    ) -> OsString {
        if let Some(raw) = self.indicator_codes.get(&indicator).cloned() {
            if raw.is_empty() {
                return self.apply_empty_style(name, wrap);
            }

            let mut style_code = String::new();
            style_code.push_str(self.reset(!self.initial_reset_is_done));
            style_code.push_str("\x1b[");
            style_code.push_str(&raw);
            style_code.push('m');

            let mut ret: OsString = style_code.into();
            ret.push(name);
            ret.push(self.reset(true));
            if wrap {
                ret.push("\x1b[K");
            }
            ret
        } else {
            let style = self.colors.style_for_indicator(indicator);
            self.apply_style(style, None, name, wrap)
        }
    }

    pub(crate) fn has_indicator_style(&self, indicator: Indicator) -> bool {
        self.indicator_codes.contains_key(&indicator)
            || self.colors.style_for_indicator(indicator).is_some()
    }

    pub(crate) fn apply_orphan_link_style(&mut self, name: OsString, wrap: bool) -> OsString {
        if self.has_indicator_style(Indicator::OrphanedSymbolicLink) {
            self.apply_indicator_style(Indicator::OrphanedSymbolicLink, name, wrap)
        } else {
            self.apply_indicator_style(Indicator::MissingFile, name, wrap)
        }
    }

    pub(crate) fn apply_missing_target_style(&mut self, name: OsString, wrap: bool) -> OsString {
        if self.has_indicator_style(Indicator::MissingFile) {
            self.apply_indicator_style(Indicator::MissingFile, name, wrap)
        } else {
            self.apply_indicator_style(Indicator::OrphanedSymbolicLink, name, wrap)
        }
    }

    fn apply_empty_style(&mut self, name: OsString, wrap: bool) -> OsString {
        let mut style_code = String::new();
        style_code.push_str(self.reset(!self.initial_reset_is_done));
        style_code.push_str("\x1b[m");

        let mut ret: OsString = style_code.into();
        ret.push(name);
        ret.push(self.reset(true));
        if wrap {
            ret.push("\x1b[K");
        }
        ret
    }

    fn color_symlink_name(
        &mut self,
        path: &PathData,
        name: OsString,
        wrap: bool,
    ) -> Option<OsString> {
        if path.must_dereference && path.metadata().is_none() {
            return None;
        }
        let mut target = path.path().read_link().ok()?;
        if target.is_relative() {
            if let Some(parent) = path.path().parent() {
                target = parent.join(target);
            }
        }

        match fs::metadata(&target) {
            Ok(metadata) => {
                if self.ln_color_from_target {
                    let style = self
                        .colors
                        .style_for_path_with_metadata(&target, Some(&metadata));
                    Some(self.apply_style(style, None, name, wrap))
                } else {
                    None
                }
            }
            Err(_) => {
                if self.ln_color_from_target {
                    Some(self.apply_orphan_link_style(name, wrap))
                } else {
                    None
                }
            }
        }
    }

    fn indicator_has(&self, indicator: Indicator) -> bool {
        self.indicator_codes.contains_key(&indicator)
    }

    fn indicator_for_raw_code(&self, path: &PathData) -> Option<Indicator> {
        if self.indicator_codes.is_empty() {
            return None;
        }

        let exists = path.path().exists();
        let Some(file_type) = path.file_type() else {
            if self.indicator_has(Indicator::MissingFile) && !exists {
                return Some(Indicator::MissingFile);
            }
            return None;
        };

        if file_type.is_symlink() {
            let orphan_style = self.indicator_codes.get(&Indicator::OrphanedSymbolicLink);
            let orphan_has_color = orphan_style.map(|s| !s.is_empty()).unwrap_or(false);
            if !exists && (orphan_has_color || self.ln_color_from_target) {
                return Some(Indicator::OrphanedSymbolicLink);
            }
            if self.indicator_has(Indicator::SymbolicLink) {
                return Some(Indicator::SymbolicLink);
            }
            if !exists && self.indicator_has(Indicator::MissingFile) {
                return Some(Indicator::MissingFile);
            }
            return None;
        }
        if self.indicator_has(Indicator::MissingFile) && !exists {
            return Some(Indicator::MissingFile);
        }

        if file_type.is_file() {
            #[cfg(unix)]
            {
                if let Some(metadata) = path.metadata() {
                    let mode = metadata.mode();
                    if self.indicator_has(Indicator::Setuid) && mode & 0o4000 != 0 {
                        return Some(Indicator::Setuid);
                    }
                    if self.indicator_has(Indicator::Setgid) && mode & 0o2000 != 0 {
                        return Some(Indicator::Setgid);
                    }
                    if self.indicator_has(Indicator::ExecutableFile) && mode & 0o0111 != 0 {
                        return Some(Indicator::ExecutableFile);
                    }
                    if self.indicator_has(Indicator::MultipleHardLinks) && metadata.nlink() > 1 {
                        return Some(Indicator::MultipleHardLinks);
                    }
                }
            }

            if self.indicator_has(Indicator::RegularFile) {
                return Some(Indicator::RegularFile);
            }
        } else if file_type.is_dir() {
            #[cfg(unix)]
            {
                if let Some(metadata) = path.metadata() {
                    let mode = metadata.mode();
                    if self.indicator_has(Indicator::StickyAndOtherWritable)
                        && mode & 0o1002 == 0o1002
                    {
                        return Some(Indicator::StickyAndOtherWritable);
                    }
                    if self.indicator_has(Indicator::OtherWritable) && mode & 0o0002 != 0 {
                        return Some(Indicator::OtherWritable);
                    }
                    if self.indicator_has(Indicator::Sticky) && mode & 0o1000 != 0 {
                        return Some(Indicator::Sticky);
                    }
                }
            }

            if self.indicator_has(Indicator::Directory) {
                return Some(Indicator::Directory);
            }
        } else {
            #[cfg(unix)]
            {
                if file_type.is_fifo() && self.indicator_has(Indicator::FIFO) {
                    return Some(Indicator::FIFO);
                }
                if file_type.is_socket() && self.indicator_has(Indicator::Socket) {
                    return Some(Indicator::Socket);
                }
                if file_type.is_block_device() && self.indicator_has(Indicator::BlockDevice) {
                    return Some(Indicator::BlockDevice);
                }
                if file_type.is_char_device() && self.indicator_has(Indicator::CharacterDevice) {
                    return Some(Indicator::CharacterDevice);
                }
            }
        }

        None
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
            return style_manager.apply_style(capabilities, Some(path), name, wrap);
        }
    }

    if target_symlink.is_none() && path.file_type().is_some_and(|ft| ft.is_symlink()) {
        if let Some(colored) = style_manager.color_symlink_name(path, name.clone(), wrap) {
            return colored;
        }
    }

    if let Some(target) = target_symlink {
        // use the optional target_symlink
        // Use fn symlink_metadata directly instead of get_metadata() here because ls
        // should not exit with an err, if we are unable to obtain the target_metadata
        return style_manager.apply_style_for_path(target, name, wrap);
    }

    if !path.must_dereference {
        // If we need to dereference (follow) a symlink, we will need to get the metadata
        // There is a DirEntry, we don't need to get the metadata for the color
        return style_manager.apply_style_for_path(path, name, wrap);
    }

    let md_option: Option<Metadata> = path
        .metadata()
        .cloned()
        .or_else(|| path.p_buf.symlink_metadata().ok());

    style_manager.apply_style_based_on_metadata(path, md_option.as_ref(), name, wrap)
}

fn parse_indicator_codes() -> (HashMap<Indicator, String>, bool) {
    let mut indicator_codes = HashMap::new();
    let mut ln_color_from_target = false;

    if let Ok(ls_colors) = env::var("LS_COLORS") {
        for entry in ls_colors.split(':') {
            if entry.is_empty() {
                continue;
            }
            let Some((key, value)) = entry.split_once('=') else {
                continue;
            };

            if let Some(indicator) = Indicator::from(key) {
                if indicator == Indicator::SymbolicLink && value == "target" {
                    ln_color_from_target = true;
                    continue;
                }
                indicator_codes.insert(indicator, canonicalize_indicator_value(value));
            }
        }
    }

    (indicator_codes, ln_color_from_target)
}

fn canonicalize_indicator_value(value: &str) -> String {
    if value.len() == 1 && value.chars().all(|c| c.is_ascii_digit()) {
        let mut canonical = String::with_capacity(2);
        canonical.push('0');
        canonical.push_str(value);
        canonical
    } else {
        value.to_string()
    }
}
