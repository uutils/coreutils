// spell-checker:ignore (words) writeback

use clap::{crate_version, App, Arg};

const ABOUT: &str = "Overwrite the specified FILE(s) repeatedly, in order to make it harder\n\
for even very expensive hardware probing to recover the data.
";

const AFTER_HELP: &str =
    "Delete FILE(s) if --remove (-u) is specified.  The default is not to remove\n\
     the files because it is common to operate on device files like /dev/hda,\n\
     and those files usually should not be removed.\n\
     \n\
     CAUTION: Note that shred relies on a very important assumption:\n\
     that the file system overwrites data in place.  This is the traditional\n\
     way to do things, but many modern file system designs do not satisfy this\n\
     assumption.  The following are examples of file systems on which shred is\n\
     not effective, or is not guaranteed to be effective in all file system modes:\n\
     \n\
     * log-structured or journal file systems, such as those supplied with\n\
     AIX and Solaris (and JFS, ReiserFS, XFS, Ext3, etc.)\n\
     \n\
     * file systems that write redundant data and carry on even if some writes\n\
     fail, such as RAID-based file systems\n\
     \n\
     * file systems that make snapshots, such as Network Appliance's NFS server\n\
     \n\
     * file systems that cache in temporary locations, such as NFS\n\
     version 3 clients\n\
     \n\
     * compressed file systems\n\
     \n\
     In the case of ext3 file systems, the above disclaimer applies\n\
     and shred is thus of limited effectiveness) only in data=journal mode,\n\
     which journals file data in addition to just metadata.  In both the\n\
     data=ordered (default) and data=writeback modes, shred works as usual.\n\
     Ext3 journal modes can be changed by adding the data=something option\n\
     to the mount options for a particular file system in the /etc/fstab file,\n\
     as documented in the mount man page (man mount).\n\
     \n\
     In addition, file system backups and remote mirrors may contain copies\n\
     of the file that cannot be removed, and that will allow a shredded file\n\
     to be recovered later.\n\
     ";

pub mod options {
    pub const FORCE: &str = "force";
    pub const FILE: &str = "file";
    pub const ITERATIONS: &str = "iterations";
    pub const SIZE: &str = "size";
    pub const REMOVE: &str = "remove";
    pub const VERBOSE: &str = "verbose";
    pub const EXACT: &str = "exact";
    pub const ZERO: &str = "zero";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .arg(
            Arg::with_name(options::FORCE)
                .long(options::FORCE)
                .short("f")
                .help("change permissions to allow writing if necessary"),
        )
        .arg(
            Arg::with_name(options::ITERATIONS)
                .long(options::ITERATIONS)
                .short("n")
                .help("overwrite N times instead of the default (3)")
                .value_name("NUMBER")
                .default_value("3"),
        )
        .arg(
            Arg::with_name(options::SIZE)
                .long(options::SIZE)
                .short("s")
                .takes_value(true)
                .value_name("N")
                .help("shred this many bytes (suffixes like K, M, G accepted)"),
        )
        .arg(
            Arg::with_name(options::REMOVE)
                .short("u")
                .long(options::REMOVE)
                .help("truncate and remove file after overwriting;  See below"),
        )
        .arg(
            Arg::with_name(options::VERBOSE)
                .long(options::VERBOSE)
                .short("v")
                .help("show progress"),
        )
        .arg(
            Arg::with_name(options::EXACT)
                .long(options::EXACT)
                .short("x")
                .help(
                    "do not round file sizes up to the next full block;\n\
                     this is the default for non-regular files",
                ),
        )
        .arg(
            Arg::with_name(options::ZERO)
                .long(options::ZERO)
                .short("z")
                .help("add a final overwrite with zeros to hide shredding"),
        )
        // Positional arguments
        .arg(Arg::with_name(options::FILE).hidden(true).multiple(true))
}
