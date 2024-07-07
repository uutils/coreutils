// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) COMFOLLOW Chowner RFILE RFILE's derefer dgid nonblank nonprint nonprinting

use uucore::display::Quotable;
use uucore::entries;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::perms::{chown_base, options, GidUidOwnerFilter, IfFrom};

use clap::ArgMatches;

use std::fs;
use std::os::unix::fs::MetadataExt;

fn parse_gid_and_uid(matches: &ArgMatches) -> UResult<GidUidOwnerFilter> {
    let mut raw_group: String = String::new();
    let dest_gid = if let Some(file) = matches.get_one::<String>(options::REFERENCE) {
        fs::metadata(file)
            .map(|meta| {
                let gid = meta.gid();
                raw_group = entries::gid2grp(gid).unwrap_or_else(|_| gid.to_string());
                Some(gid)
            })
            .map_err_context(|| format!("failed to get attributes of {}", file.quote()))?
    } else {
        let group = matches
            .get_one::<String>(options::ARG_GROUP)
            .map(|s| s.as_str())
            .unwrap_or_default();
        raw_group = group.to_string();
        if group.is_empty() {
            None
        } else {
            match entries::grp2gid(group) {
                Ok(g) => Some(g),
                _ => {
                    return Err(USimpleError::new(
                        1,
                        format!("invalid group: {}", group.quote()),
                    ))
                }
            }
        }
    };
    Ok(GidUidOwnerFilter {
        dest_gid,
        dest_uid: None,
        raw_owner: raw_group,
        filter: IfFrom::All,
    })
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    chown_base(
        crate::uu_app(),
        args,
        options::ARG_GROUP,
        parse_gid_and_uid,
        true,
    )
}
