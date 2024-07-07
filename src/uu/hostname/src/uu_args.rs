// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{builder::ValueParser, crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("hostname.md");
const USAGE: &str = help_usage!("hostname.md");

pub mod options {
    pub static OPT_DOMAIN: &str = "domain";
    pub static OPT_IP_ADDRESS: &str = "ip-address";
    pub static OPT_FQDN: &str = "fqdn";
    pub static OPT_SHORT: &str = "short";
    pub static OPT_HOST: &str = "host";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::OPT_DOMAIN)
                .short('d')
                .long("domain")
                .overrides_with_all([
                    options::OPT_DOMAIN,
                    options::OPT_IP_ADDRESS,
                    options::OPT_FQDN,
                    options::OPT_SHORT,
                ])
                .help("Display the name of the DNS domain if possible")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_IP_ADDRESS)
                .short('i')
                .long("ip-address")
                .overrides_with_all([
                    options::OPT_DOMAIN,
                    options::OPT_IP_ADDRESS,
                    options::OPT_FQDN,
                    options::OPT_SHORT,
                ])
                .help("Display the network address(es) of the host")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_FQDN)
                .short('f')
                .long("fqdn")
                .overrides_with_all([
                    options::OPT_DOMAIN,
                    options::OPT_IP_ADDRESS,
                    options::OPT_FQDN,
                    options::OPT_SHORT,
                ])
                .help("Display the FQDN (Fully Qualified Domain Name) (default)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_SHORT)
                .short('s')
                .long("short")
                .overrides_with_all([
                    options::OPT_DOMAIN,
                    options::OPT_IP_ADDRESS,
                    options::OPT_FQDN,
                    options::OPT_SHORT,
                ])
                .help("Display the short hostname (the portion before the first dot) if possible")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_HOST)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::Hostname),
        )
}
