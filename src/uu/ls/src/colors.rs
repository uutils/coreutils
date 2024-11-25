// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use super::get_metadata_with_deref_opt;
use super::PathData;
use lscolors::{Indicator, LsColors, Style};
use std::fs::{DirEntry, Metadata};
use std::io::{BufWriter, Stdout};

/// We need this struct to be able to store the previous style.
/// This because we need to check the previous value in case we don't need
/// the reset
pub(crate) struct StyleManager<'a> {
    /// last style that is applied, if `None` that means reset is applied.
    pub(crate) current_style: Option<Style>,
    /// `true` if the initial reset is applied
    pub(crate) initial_reset_is_done: bool,
    pub(crate) colors: &'a LsColors,
}

impl<'a> StyleManager<'a> {
    pub(crate) fn new(colors: &'a LsColors) -> Self {
        Self {
            initial_reset_is_done: false,
            current_style: None,
            colors,
        }
    }

    pub(crate) fn apply_style(
        &mut self,
        new_style: Option<&Style>,
        name: &str,
        wrap: bool,
    ) -> String {
        let mut style_code = String::new();
        let mut force_suffix_reset: bool = false;

        // if reset is done we need to apply normal style before applying new style
        if self.is_reset() {
            if let Some(norm_sty) = self.get_normal_style().copied() {
                style_code.push_str(&self.get_style_code(&norm_sty));
            }
        }

        if let Some(new_style) = new_style {
            // we only need to apply a new style if it's not the same as the current
            // style for example if normal is the current style and a file with
            // normal style is to be printed we could skip printing new color
            // codes
            if !self.is_current_style(new_style) {
                style_code.push_str(&self.reset(!self.initial_reset_is_done));
                style_code.push_str(&self.get_style_code(new_style));
            }
        }
        // if new style is None and current style is Normal we should reset it
        else if matches!(self.get_normal_style().copied(), Some(norm_style) if self.is_current_style(&norm_style))
        {
            style_code.push_str(&self.reset(false));
            // even though this is an unnecessary reset for gnu compatibility we allow it here
            force_suffix_reset = true;
        }

        // we need this clear to eol code in some terminals, for instance if the
        // text is in the last row of the terminal meaning the terminal need to
        // scroll up in order to print new text in this situation if the clear
        // to eol code is not present the background of the text would stretch
        // till the end of line
        let clear_to_eol = if wrap { "\x1b[K" } else { "" };

        format!(
            "{style_code}{name}{}{clear_to_eol}",
            self.reset(force_suffix_reset),
        )
    }

    /// Resets the current style and returns the default ANSI reset code to
    /// reset all text formatting attributes. If `force` is true, the reset is
    /// done even if the reset has been applied before.
    pub(crate) fn reset(&mut self, force: bool) -> String {
        // todo:
        // We need to use style from `Indicator::Reset` but as of now ls colors
        // uses a fallback mechanism and because of that if `Indicator::Reset`
        // is not specified it would fallback to `Indicator::Normal` which seems
        // to be non compatible with gnu
        if self.current_style.is_some() || force {
            self.initial_reset_is_done = true;
            self.current_style = None;
            return "\x1b[0m".to_string();
        }
        String::new()
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

    pub(crate) fn is_current_style(&mut self, new_style: &Style) -> bool {
        matches!(&self.current_style,Some(style) if style == new_style )
    }

    pub(crate) fn is_reset(&mut self) -> bool {
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
        name: &str,
        wrap: bool,
    ) -> String {
        let style = self
            .colors
            .style_for_path_with_metadata(&path.p_buf, md_option);
        self.apply_style(style, name, wrap)
    }

    pub(crate) fn apply_style_based_on_dir_entry(
        &mut self,
        dir_entry: &DirEntry,
        name: &str,
        wrap: bool,
    ) -> String {
        let style = self.colors.style_for(dir_entry);
        self.apply_style(style, name, wrap)
    }
}

/// Colors the provided name based on the style determined for the given path
/// This function is quite long because it tries to leverage DirEntry to avoid
/// unnecessary calls to stat()
/// and manages the symlink errors
pub(crate) fn color_name(
    name: &str,
    path: &PathData,
    style_manager: &mut StyleManager,
    out: &mut BufWriter<Stdout>,
    target_symlink: Option<&PathData>,
    wrap: bool,
) -> String {
    if !path.must_dereference {
        // If we need to dereference (follow) a symlink, we will need to get the metadata
        if let Some(de) = &path.de {
            // There is a DirEntry, we don't need to get the metadata for the color
            return style_manager.apply_style_based_on_dir_entry(de, name, wrap);
        }
    }

    if let Some(target) = target_symlink {
        // use the optional target_symlink
        // Use fn get_metadata_with_deref_opt instead of get_metadata() here because ls
        // should not exit with an err, if we are unable to obtain the target_metadata
        let md_res = get_metadata_with_deref_opt(&target.p_buf, path.must_dereference);
        let md = md_res.or_else(|_| path.p_buf.symlink_metadata());
        style_manager.apply_style_based_on_metadata(path, md.ok().as_ref(), name, wrap)
    } else {
        let md_option = path.get_metadata(out);
        let symlink_metadata = path.p_buf.symlink_metadata().ok();
        let md = md_option.or(symlink_metadata.as_ref());
        style_manager.apply_style_based_on_metadata(path, md, name, wrap)
    }
}
