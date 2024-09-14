// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// cSpell:disable

//! Provides color handling for `ls` and other utilities.

/// The keywords COLOR, OPTIONS, and EIGHTBIT (honored by the
/// slackware version of dircolors) are recognized but ignored.
/// Global config options can be specified before TERM or COLORTERM entries
/// below are TERM or COLORTERM entries, which can be glob patterns, which
/// restrict following config to systems with matching environment variables.
pub static TERMS: &[&str] = &[
    "Eterm",
    "ansi",
    "*color*",
    "con[0-9]*x[0-9]*",
    "cons25",
    "console",
    "cygwin",
    "*direct*",
    "dtterm",
    "gnome",
    "hurd",
    "jfbterm",
    "konsole",
    "kterm",
    "linux",
    "linux-c",
    "mlterm",
    "putty",
    "rxvt*",
    "screen*",
    "st",
    "terminator",
    "tmux*",
    "vt100",
    "xterm*",
];

/// Below are the color init strings for the basic file types.
/// One can use codes for 256 or more colors supported by modern terminals.
/// The default color codes use the capabilities of an 8 color terminal
/// with some additional attributes as per the following codes:
/// Attribute codes:
/// 00=none 01=bold 04=underscore 05=blink 07=reverse 08=concealed
/// Text color codes:
/// 30=black 31=red 32=green 33=yellow 34=blue 35=magenta 36=cyan 37=white
/// Background color codes:
/// 40=black 41=red 42=green 43=yellow 44=blue 45=magenta 46=cyan 47=white
/// #NORMAL 00 /// no color code at all
/// #FILE 00 /// regular file: use no color at all
pub static FILE_TYPES: &[(&str, &str, &str)] = &[
    ("RESET", "rs", "0"),                     // reset to "normal" color
    ("DIR", "di", "01;34"),                   // directory
    ("LINK", "ln", "01;36"),                  // symbolic link
    ("MULTIHARDLINK", "mh", "00"),            // regular file with more than one link
    ("FIFO", "pi", "40;33"),                  // pipe
    ("SOCK", "so", "01;35"),                  // socket
    ("DOOR", "do", "01;35"),                  // door
    ("BLK", "bd", "40;33;01"),                // block device driver
    ("CHR", "cd", "40;33;01"),                // character device driver
    ("ORPHAN", "or", "40;31;01"),             // symlink to nonexistent file, or non-stat'able file
    ("MISSING", "mi", "00"),                  // ... and the files they point to
    ("SETUID", "su", "37;41"),                // file that is setuid (u+s)
    ("SETGID", "sg", "30;43"),                // file that is setgid (g+s)
    ("CAPABILITY", "ca", "00"),               // file with capability
    ("STICKY_OTHER_WRITABLE", "tw", "30;42"), // dir that is sticky and other-writable (+t,o+w)
    ("OTHER_WRITABLE", "ow", "34;42"),        // dir that is other-writable (o+w) and not sticky
    ("STICKY", "st", "37;44"), // dir with the sticky bit set (+t) and not other-writable
    ("EXEC", "ex", "01;32"),   // files with execute permission
];

/// Colors for file types
///
/// List any file extensions like '.gz' or '.tar' that you would like ls
/// to color below. Put the extension, a space, and the color init string.
/// (and any comments you want to add after a '#')
pub static FILE_COLORS: &[(&str, &str)] = &[
    /*
    // Executables (Windows)
    (".cmd", "01;32"),
    (".exe", "01;32"),
    (".com", "01;32"),
    (".btm", "01;32"),
    (".bat", "01;32"),
    (".sh", "01;32"),
    (".csh", "01;32"),*/
    // Archives or compressed
    (".tar", "01;31"),
    (".tgz", "01;31"),
    (".arc", "01;31"),
    (".arj", "01;31"),
    (".taz", "01;31"),
    (".lha", "01;31"),
    (".lz4", "01;31"),
    (".lzh", "01;31"),
    (".lzma", "01;31"),
    (".tlz", "01;31"),
    (".txz", "01;31"),
    (".tzo", "01;31"),
    (".t7z", "01;31"),
    (".zip", "01;31"),
    (".z", "01;31"),
    (".dz", "01;31"),
    (".gz", "01;31"),
    (".lrz", "01;31"),
    (".lz", "01;31"),
    (".lzo", "01;31"),
    (".xz", "01;31"),
    (".zst", "01;31"),
    (".tzst", "01;31"),
    (".bz2", "01;31"),
    (".bz", "01;31"),
    (".tbz", "01;31"),
    (".tbz2", "01;31"),
    (".tz", "01;31"),
    (".deb", "01;31"),
    (".rpm", "01;31"),
    (".jar", "01;31"),
    (".war", "01;31"),
    (".ear", "01;31"),
    (".sar", "01;31"),
    (".rar", "01;31"),
    (".alz", "01;31"),
    (".ace", "01;31"),
    (".zoo", "01;31"),
    (".cpio", "01;31"),
    (".7z", "01;31"),
    (".rz", "01;31"),
    (".cab", "01;31"),
    (".wim", "01;31"),
    (".swm", "01;31"),
    (".dwm", "01;31"),
    (".esd", "01;31"),
    // Image formats
    (".avif", "01;35"),
    (".jpg", "01;35"),
    (".jpeg", "01;35"),
    (".mjpg", "01;35"),
    (".mjpeg", "01;35"),
    (".gif", "01;35"),
    (".bmp", "01;35"),
    (".pbm", "01;35"),
    (".pgm", "01;35"),
    (".ppm", "01;35"),
    (".tga", "01;35"),
    (".xbm", "01;35"),
    (".xpm", "01;35"),
    (".tif", "01;35"),
    (".tiff", "01;35"),
    (".png", "01;35"),
    (".svg", "01;35"),
    (".svgz", "01;35"),
    (".mng", "01;35"),
    (".pcx", "01;35"),
    (".mov", "01;35"),
    (".mpg", "01;35"),
    (".mpeg", "01;35"),
    (".m2v", "01;35"),
    (".mkv", "01;35"),
    (".webm", "01;35"),
    (".webp", "01;35"),
    (".ogm", "01;35"),
    (".mp4", "01;35"),
    (".m4v", "01;35"),
    (".mp4v", "01;35"),
    (".vob", "01;35"),
    (".qt", "01;35"),
    (".nuv", "01;35"),
    (".wmv", "01;35"),
    (".asf", "01;35"),
    (".rm", "01;35"),
    (".rmvb", "01;35"),
    (".flc", "01;35"),
    (".avi", "01;35"),
    (".fli", "01;35"),
    (".flv", "01;35"),
    (".gl", "01;35"),
    (".dl", "01;35"),
    (".xcf", "01;35"),
    (".xwd", "01;35"),
    (".yuv", "01;35"),
    (".cgm", "01;35"),
    (".emf", "01;35"),
    // https://wiki.xiph.org/MIME_Types_and_File_Extensions
    (".ogv", "01;35"),
    (".ogx", "01;35"),
    // Audio formats
    (".aac", "00;36"),
    (".au", "00;36"),
    (".flac", "00;36"),
    (".m4a", "00;36"),
    (".mid", "00;36"),
    (".midi", "00;36"),
    (".mka", "00;36"),
    (".mp3", "00;36"),
    (".mpc", "00;36"),
    (".ogg", "00;36"),
    (".ra", "00;36"),
    (".wav", "00;36"),
    // https://wiki.xiph.org/MIME_Types_and_File_Extensions
    (".oga", "00;36"),
    (".opus", "00;36"),
    (".spx", "00;36"),
    (".xspf", "00;36"),
    // Backup files
    ("*~", "00;90"),
    ("*#", "00;90"),
    (".bak", "00;90"),
    (".old", "00;90"),
    (".orig", "00;90"),
    (".part", "00;90"),
    (".rej", "00;90"),
    (".swp", "00;90"),
    (".tmp", "00;90"),
    (".dpkg-dist", "00;90"),
    (".dpkg-old", "00;90"),
    (".ucf-dist", "00;90"),
    (".ucf-new", "00;90"),
    (".ucf-old", "00;90"),
    (".rpmnew", "00;90"),
    (".rpmorig", "00;90"),
    (".rpmsave", "00;90"),
];

/// Below are the terminal color capabilities
pub static FILE_ATTRIBUTE_CODES: &[(&str, &str)] = &[
    ("normal", "no"),
    ("norm", "no"),
    ("file", "fi"),
    ("reset", "rs"),
    ("dir", "di"),
    ("lnk", "ln"),
    ("link", "ln"),
    ("symlink", "ln"),
    ("orphan", "or"),
    ("missing", "mi"),
    ("fifo", "pi"),
    ("pipe", "pi"),
    ("sock", "so"),
    ("blk", "bd"),
    ("block", "bd"),
    ("chr", "cd"),
    ("char", "cd"),
    ("door", "do"),
    ("exec", "ex"),
    ("left", "lc"),
    ("leftcode", "lc"),
    ("right", "rc"),
    ("rightcode", "rc"),
    ("end", "ec"),
    ("endcode", "ec"),
    ("suid", "su"),
    ("setuid", "su"),
    ("sgid", "sg"),
    ("setgid", "sg"),
    ("sticky", "st"),
    ("other_writable", "ow"),
    ("owr", "ow"),
    ("sticky_other_writable", "tw"),
    ("owt", "tw"),
    ("capability", "ca"),
    ("multihardlink", "mh"),
    ("clrtoeol", "cl"),
];
