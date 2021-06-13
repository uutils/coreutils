use clap::{crate_version, App, Arg};

const ABOUT: &str = "Display or set the system's host name.";

pub const OPT_DOMAIN: &str = "domain";
pub const OPT_IP_ADDRESS: &str = "ip-address";
pub const OPT_FQDN: &str = "fqdn";
pub const OPT_SHORT: &str = "short";
pub const OPT_HOST: &str = "host";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(OPT_DOMAIN)
                .short("d")
                .long("domain")
                .help("Display the name of the DNS domain if possible"),
        )
        .arg(
            Arg::with_name(OPT_IP_ADDRESS)
                .short("i")
                .long("ip-address")
                .help("Display the network address(es) of the host"),
        )
        // TODO: support --long
        .arg(
            Arg::with_name(OPT_FQDN)
                .short("f")
                .long("fqdn")
                .help("Display the FQDN (Fully Qualified Domain Name) (default)"),
        )
        .arg(Arg::with_name(OPT_SHORT).short("s").long("short").help(
            "Display the short hostname (the portion before the first dot) if \
             possible",
        ))
        .arg(Arg::with_name(OPT_HOST))
}
