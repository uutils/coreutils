#![crate_name = "uu_ls"]

// This file is part of the uutils coreutils package.
//
// (c) Jeremiah Peschka <jeremiah.peschka@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

extern crate getopts;
extern crate pretty_bytes;
extern crate termsize;
extern crate term_grid;
extern crate time;
extern crate unicode_width;
use pretty_bytes::converter::convert;
use term_grid::{Grid, GridOptions, Direction, Filling, Cell};
use time::{Timespec, strftime};

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate uucore;
#[cfg(unix)]
use uucore::libc::{S_ISUID, S_ISGID, S_ISVTX, S_IRUSR, S_IWUSR, S_IXUSR, S_IRGRP, S_IWGRP, S_IXGRP,
                   S_IROTH, S_IWOTH, S_IXOTH, mode_t};

use std::fs;
use std::fs::{DirEntry, FileType, Metadata};
use std::path::{Path, PathBuf};
use std::io::Write;
use std::cmp::Reverse;
#[cfg(unix)]
use std::collections::HashMap;

#[cfg(any(unix, target_os = "redox"))]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
#[cfg(unix)]
use unicode_width::UnicodeWidthStr;

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

static NAME: &'static str = "ls";
static SUMMARY: &'static str = "";
static LONG_HELP: &'static str = "
 By default, ls will list the files and contents of any directories on
 the command line, expect that it will ignore files and directories
 whose names start with '.'
";

#[cfg(unix)]
static DEFAULT_COLORS: &'static str = "rs=0:di=01;34:ln=01;36:mh=00:pi=40;33:so=01;35:do=01;35:bd=40;33;01:cd=40;33;01:or=40;31;01:mi=00:su=37;41:sg=30;43:ca=30;41:tw=30;42:ow=34;42:st=37;44:ex=01;32:*.tar=01;31:*.tgz=01;31:*.arc=01;31:*.arj=01;31:*.taz=01;31:*.lha=01;31:*.lz4=01;31:*.lzh=01;31:*.lzma=01;31:*.tlz=01;31:*.txz=01;31:*.tzo=01;31:*.t7z=01;31:*.zip=01;31:*.z=01;31:*.Z=01;31:*.dz=01;31:*.gz=01;31:*.lrz=01;31:*.lz=01;31:*.lzo=01;31:*.xz=01;31:*.bz2=01;31:*.bz=01;31:*.tbz=01;31:*.tbz2=01;31:*.tz=01;31:*.deb=01;31:*.rpm=01;31:*.jar=01;31:*.war=01;31:*.ear=01;31:*.sar=01;31:*.rar=01;31:*.alz=01;31:*.ace=01;31:*.zoo=01;31:*.cpio=01;31:*.7z=01;31:*.rz=01;31:*.cab=01;31:*.jpg=01;35:*.jpeg=01;35:*.gif=01;35:*.bmp=01;35:*.pbm=01;35:*.pgm=01;35:*.ppm=01;35:*.tga=01;35:*.xbm=01;35:*.xpm=01;35:*.tif=01;35:*.tiff=01;35:*.png=01;35:*.svg=01;35:*.svgz=01;35:*.mng=01;35:*.pcx=01;35:*.mov=01;35:*.mpg=01;35:*.mpeg=01;35:*.m2v=01;35:*.mkv=01;35:*.webm=01;35:*.ogm=01;35:*.mp4=01;35:*.m4v=01;35:*.mp4v=01;35:*.vob=01;35:*.qt=01;35:*.nuv=01;35:*.wmv=01;35:*.asf=01;35:*.rm=01;35:*.rmvb=01;35:*.flc=01;35:*.avi=01;35:*.fli=01;35:*.flv=01;35:*.gl=01;35:*.dl=01;35:*.xcf=01;35:*.xwd=01;35:*.yuv=01;35:*.cgm=01;35:*.emf=01;35:*.ogv=01;35:*.ogx=01;35:*.aac=00;36:*.au=00;36:*.flac=00;36:*.m4a=00;36:*.mid=00;36:*.midi=00;36:*.mka=00;36:*.mp3=00;36:*.mpc=00;36:*.ogg=00;36:*.ra=00;36:*.wav=00;36:*.oga=00;36:*.opus=00;36:*.spx=00;36:*.xspf=00;36:";

#[cfg(unix)]
lazy_static! {
    static ref LS_COLORS: String = std::env::var("LS_COLORS").unwrap_or(DEFAULT_COLORS.to_string());
    static ref COLOR_MAP: HashMap<&'static str, &'static str> = {
        let codes = LS_COLORS.split(":");
        let mut map = HashMap::new();
        for c in codes {
            let p: Vec<_> = c.split("=").collect();
            if p.len() == 2 {
                map.insert(p[0], p[1]);
            }
        }
        map
    };
    static ref RESET_CODE: &'static str = COLOR_MAP.get("rs").unwrap_or(&"0");
    static ref LEFT_CODE: &'static str = COLOR_MAP.get("lc").unwrap_or(&"\x1b[");
    static ref RIGHT_CODE: &'static str = COLOR_MAP.get("rc").unwrap_or(&"m");
    static ref END_CODE: &'static str = COLOR_MAP.get("ec").unwrap_or(&"");
}

pub fn uumain(args: Vec<String>) -> i32 {
    let syntax = format!("[OPTION]... DIRECTORY
 {0} [OPTION]... [FILE]...", NAME);
    let matches = new_coreopts!(&syntax, SUMMARY, LONG_HELP)
        .optflag("a",
                 "all",
                 "Do not ignore hidden files (files with names that start with '.').")
        .optflag("A",
                 "almost-all",
                 "In a directory, do not ignore all file names that start with '.', only ignore \
                  '.' and '..'.")
        .optflag("B",
                 "ignore-backups",
                 "Ignore entries which end with ~.")
        .optflag("c",
                 "",
                 "If the long listing format (e.g., -l, -o) is being used, print the status \
                 change time (the ‘ctime’ in the inode) instead of the modification time. When \
                 explicitly sorting by time (--sort=time or -t) or when not using a long listing \
                 format, sort according to the status change time.")
        .optflag("d",
                 "directory",
                 "Only list the names of directories, rather than listing directory contents. \
                  This will not follow symbolic links unless one of `--dereference-command-line \
                  (-H)`, `--dereference (-L)`, or `--dereference-command-line-symlink-to-dir` is \
                  specified.")
        .optflag("F",
                 "classify",
                 "Append a character to each file name indicating the file type. Also, for \
                 regular files that are executable, append '*'. The file type indicators are \
                 '/' for directories, '@' for symbolic links, '|' for FIFOs, '=' for sockets, \
                 '>' for doors, and nothing for regular files.")
        .optflag("h",
                 "human-readable",
                 "Print human readable file sizes (e.g. 1K 234M 56G).")
        .optflag("i",
                 "inode",
                 "print the index number of each file")
        .optflag("L",
                 "dereference",
                 "When showing file information for a symbolic link, show information for the \
                 file the link references rather than the link itself.")
        .optflag("l", "long", "Display detailed information.")
        .optflag("n", "numeric-uid-gid", "-l with numeric UIDs and GIDs.")
        .optflag("r",
                 "reverse",
                 "Reverse whatever the sorting method is--e.g., list files in reverse \
                 alphabetical order, youngest first, smallest first, or whatever.")
        .optflag("R",
                 "recursive",
                 "List the contents of all directories recursively.")
        .optflag("S", "", "Sort by file size, largest first.")
        .optflag("t",
                 "",
                 "Sort by modification time (the 'mtime' in the inode), newest first.")
        .optflag("U",
                 "",
                 "Do not sort; list the files in whatever order they are stored in the \
                 directory.  This is especially useful when listing very large directories, \
                 since not doing any sorting can be noticeably faster.")
        .optflag("", "color", "Color output based on file type.")
        .parse(args);

    list(matches);
    0
}

fn list(options: getopts::Matches) {
    let locs: Vec<String> = if options.free.is_empty() {
        vec![String::from(".")]
    } else {
        options.free.iter().cloned().collect()
    };

    let mut files = Vec::<PathBuf>::new();
    let mut dirs = Vec::<PathBuf>::new();
    for loc in locs {
        let p = PathBuf::from(&loc);
        let mut dir = false;

        if p.is_dir() && !options.opt_present("d") {
            dir = true;
            if options.opt_present("l") && !(options.opt_present("L")) {
                if let Ok(md) = p.symlink_metadata() {
                    if md.file_type().is_symlink() {
                        dir = false;
                    }
                }
            }
        }
        if dir {
            dirs.push(p);
        } else {
            files.push(p);
        }
    }
    sort_entries(&mut files, &options);
    display_items(&files, None, &options);

    sort_entries(&mut dirs, &options);
    for dir in dirs {
        if options.free.len() > 1 {
            println!("\n{}:", dir.to_string_lossy());
        }
        enter_directory(&dir, &options);
    }
}

#[cfg(any(unix, target_os = "redox"))]
fn sort_entries(entries: &mut Vec<PathBuf>, options: &getopts::Matches) {
    let mut reverse = options.opt_present("r");
    if options.opt_present("t") {
        if options.opt_present("c") {
            entries.sort_by_key(|k| {
                Reverse(get_metadata(k, options)
                    .map(|md| md.ctime())
                    .unwrap_or(0))
            });
        } else {
            entries.sort_by_key(|k| {
                // Newest first
                Reverse(get_metadata(k, options)
                    .and_then(|md| md.modified())
                    .unwrap_or(std::time::UNIX_EPOCH))
            });
        }
    } else if options.opt_present("S") {
        entries.sort_by_key(|k| get_metadata(k, options).map(|md| md.size()).unwrap_or(0));
        reverse = !reverse;
    } else if !options.opt_present("U") {
        entries.sort();
    }

    if reverse {
        entries.reverse();
    }
}

#[cfg(windows)]
fn sort_entries(entries: &mut Vec<PathBuf>, options: &getopts::Matches) {
    let mut reverse = options.opt_present("r");
    if options.opt_present("t") {
        entries.sort_by_key(|k| {
            // Newest first
            Reverse(get_metadata(k, options)
                .and_then(|md| md.modified())
                .unwrap_or(std::time::UNIX_EPOCH))
        });
    } else if options.opt_present("S") {
        entries.sort_by_key(|k| get_metadata(k, options).map(|md| md.file_size()).unwrap_or(0));
        reverse = !reverse;
    } else if !options.opt_present("U") {
        entries.sort();
    }

    if reverse {
        entries.reverse();
    }
}

fn max(lhs: usize, rhs: usize) -> usize {
    if lhs > rhs {
        lhs
    } else {
        rhs
    }
}

fn should_display(entry: &DirEntry, options: &getopts::Matches) -> bool {
    let ffi_name = entry.file_name();
    let name = ffi_name.to_string_lossy();
    if !options.opt_present("a") && !options.opt_present("A") {
        if name.starts_with('.') {
            return false;
        }
    }
    if options.opt_present("B") && name.ends_with('~') {
        return false;
    }
    return true;
}

fn enter_directory(dir: &PathBuf, options: &getopts::Matches) {
    let mut entries = safe_unwrap!(fs::read_dir(dir)
        .and_then(|e| e.collect::<Result<Vec<_>, _>>()));

    entries.retain(|e| should_display(e, options));

    let mut entries: Vec<_> = entries.iter().map(DirEntry::path).collect();
    sort_entries(&mut entries, options);



    if options.opt_present("a") {
        let mut display_entries = entries.clone();
        display_entries.insert(0, dir.join(".."));
        display_entries.insert(0, dir.join("."));
        display_items(&display_entries, Some(dir), options);
    }
    else
    {
        display_items(&entries, Some(dir), options);
    }


    if options.opt_present("R") {
        for e in entries.iter().filter(|p| p.is_dir()) {
            println!("\n{}:", e.to_string_lossy());
            enter_directory(&e, options);
        }
    }
}

fn get_metadata(entry: &PathBuf, options: &getopts::Matches) -> std::io::Result<Metadata> {
    if options.opt_present("L") {
        entry.metadata().or(entry.symlink_metadata())
    } else {
        entry.symlink_metadata()
    }
}

fn display_dir_entry_size(entry: &PathBuf, options: &getopts::Matches) -> (usize, usize) {
    if let Ok(md) = get_metadata(entry, options) {
        (display_symlink_count(&md).len(), display_file_size(&md, options).len())
    } else {
        (0, 0)
    }
}

fn pad_left(string: String, count: usize) -> String {
    if count > string.len() {
        let pad = count - string.len();
        let pad = String::from_utf8(vec![' ' as u8; pad]).unwrap();
        format!("{}{}", pad, string)
    } else {
        string
    }
}

fn display_items(items: &Vec<PathBuf>, strip: Option<&Path>, options: &getopts::Matches) {
    if options.opt_present("long") || options.opt_present("numeric-uid-gid") {
        let (mut max_links, mut max_size) = (1, 1);
        for item in items {
            let (links, size) = display_dir_entry_size(item, options);
            max_links = max(links, max_links);
            max_size = max(size, max_size);
        }
        for item in items {
            display_item_long(item, strip, max_links, max_size, options);
        }
    } else {
        let names: Vec<_> = items.iter()
            .filter_map(|i| {
                let md = get_metadata(i, options);
                match md {
                    Err(e) => {
                        let filename = get_file_name(i, strip);
                        show_error!("{}: {}", filename, e);
                        None
                    }
                    Ok(md) => Some(display_file_name(&i, strip, &md, options)),
                }
            })
            .collect();
        if let Some(size) = termsize::get() {
            let mut grid = Grid::new(GridOptions {
                filling: Filling::Spaces(2),
                direction: Direction::TopToBottom,
            });
            for name in names {
                grid.add(name);
            }
            if let Some(output) = grid.fit_into_width(size.cols as usize) {
                print!("{}", output);
                return;
            }
        }

        // Couldn't display a grid, either because we don't know
        // the terminal width or because fit_into_width failed
        for i in items {
            let md = get_metadata(i, options);
            if let Ok(md) = md {
                println!("{}", display_file_name(&i, strip, &md, options).contents);
            }
        }
    }
}

fn display_item_long(item: &PathBuf,
                     strip: Option<&Path>,
                     max_links: usize,
                     max_size: usize,
                     options: &getopts::Matches) {
    let md = match get_metadata(item, options) {
        Err(e) => {
            let filename = get_file_name(&item, strip);
            show_error!("{}: {}", filename, e);
            return;
        }
        Ok(md) => md,
    };

    println!("{}{}{} {} {} {} {} {} {}",
             get_inode(&md, options),
             display_file_type(md.file_type()),
             display_permissions(&md),
             pad_left(display_symlink_count(&md), max_links),
             display_uname(&md, options),
             display_group(&md, options),
             pad_left(display_file_size(&md, options), max_size),
             display_date(&md, options),
             display_file_name(&item, strip, &md, options).contents);
}

#[cfg(unix)]
fn get_inode(metadata: &Metadata, options: &getopts::Matches) -> String {
    if options.opt_present("inode") {
        format!("{:8} ", metadata.ino())
    } else {
        "".to_string()
    }
}

#[cfg(not(unix))]
fn get_inode(_metadata: &Metadata, _options: &getopts::Matches) -> String {
    "".to_string()
}


// Currently getpwuid is `linux` target only. If it's broken out into
// a posix-compliant attribute this can be updated...
#[cfg(unix)]
use uucore::entries;

#[cfg(unix)]
fn display_uname(metadata: &Metadata, options: &getopts::Matches) -> String {
    if options.opt_present("numeric-uid-gid") {
        metadata.uid().to_string()
    } else {
        entries::uid2usr(metadata.uid()).unwrap_or(metadata.uid().to_string())
    }
}

#[cfg(unix)]
fn display_group(metadata: &Metadata, options: &getopts::Matches) -> String {
    if options.opt_present("numeric-uid-gid") {
        metadata.gid().to_string()
    } else {
        entries::gid2grp(metadata.gid()).unwrap_or(metadata.gid().to_string())
    }
}

#[cfg(not(unix))]
#[allow(unused_variables)]
fn display_uname(metadata: &Metadata, _options: &getopts::Matches) -> String {
    "somebody".to_string()
}

#[cfg(not(unix))]
#[allow(unused_variables)]
fn display_group(metadata: &Metadata, _options: &getopts::Matches) -> String {
    "somegroup".to_string()
}

#[cfg(unix)]
fn display_date(metadata: &Metadata, options: &getopts::Matches) -> String {
    let secs = if options.opt_present("c") {
        metadata.ctime()
    } else {
        metadata.mtime()
    };
    let time = time::at(Timespec::new(secs, 0));
    strftime("%F %R", &time).unwrap()
}

#[cfg(not(unix))]
#[allow(unused_variables)]
fn display_date(metadata: &Metadata, options: &getopts::Matches) -> String {
    if let Ok(mtime) = metadata.modified() {
        let time =
            time::at(Timespec::new(mtime.duration_since(std::time::UNIX_EPOCH)
                                       .unwrap()
                                       .as_secs() as i64,
                                   0));
        strftime("%F %R", &time).unwrap()
    } else {
        "???".to_string()
    }
}

fn display_file_size(metadata: &Metadata, options: &getopts::Matches) -> String {
    if options.opt_present("human-readable") {
        convert(metadata.len() as f64)
    } else {
        metadata.len().to_string()
    }
}

fn display_file_type(file_type: FileType) -> String {
    if file_type.is_dir() {
        "d".to_string()
    } else if file_type.is_symlink() {
        "l".to_string()
    } else {
        "-".to_string()
    }
}

fn get_file_name(name: &Path, strip: Option<&Path>) -> String {
    let mut name = match strip {
        Some(prefix) => name.strip_prefix(prefix).unwrap_or(name),
        None => name,
    };
    if name.as_os_str().len() == 0 {
        name = Path::new(".");
    }
    name.to_string_lossy().into_owned()
}

#[cfg(not(unix))]
fn display_file_name(path: &Path,
                     strip: Option<&Path>,
                     metadata: &Metadata,
                     options: &getopts::Matches)
                     -> Cell {
    let mut name = get_file_name(path, strip);

    if !options.opt_present("long") {
        name = get_inode(metadata, options) + &name;
    }

    if options.opt_present("classify") {
        let file_type = metadata.file_type();
        if file_type.is_dir() {
            name.push('/');
        } else if file_type.is_symlink() {
            name.push('@');
        }
    }

    if options.opt_present("long") && metadata.file_type().is_symlink() {
        if let Ok(target) = path.read_link() {
            // We don't bother updating width here because it's not used for long listings
            let target_name = target.to_string_lossy().to_string();
            name.push_str(" -> ");
            name.push_str(&target_name);
        }
    }

    name.into()
}

#[cfg(unix)]
fn color_name(name: String, typ: &str) -> String {
    let mut typ = typ;
    if !COLOR_MAP.contains_key(typ) {
        if typ == "or" {
            typ = "ln";
        } else if typ == "mi" {
            typ = "fi";
        }
    };
    if let Some(code) = COLOR_MAP.get(typ) {
        format!("{}{}{}{}{}{}{}{}",
                *LEFT_CODE,
                code,
                *RIGHT_CODE,
                name,
                *END_CODE,
                *LEFT_CODE,
                *RESET_CODE,
                *RIGHT_CODE,
        )
    } else {
        name
    }
}

macro_rules! has {
    ($mode:expr, $perm:expr) => (
        $mode & ($perm as mode_t) != 0
    )
}
#[cfg(unix)]
fn display_file_name(path: &Path,
                     strip: Option<&Path>,
                     metadata: &Metadata,
                     options: &getopts::Matches)
                     -> Cell {
    let mut name = get_file_name(path, strip);
    if !options.opt_present("long") {
        name = get_inode(metadata, options) + &name;
    }
    let mut width = UnicodeWidthStr::width(&*name);

    let color = options.opt_present("color");
    let classify = options.opt_present("classify");
    let ext;

    if color || classify {
        let file_type = metadata.file_type();

        let (code, sym) = if file_type.is_dir() {
            ("di", Some('/'))
        } else if file_type.is_symlink() {
            if path.exists() {
                ("ln", Some('@'))
            } else {
                ("or", Some('@'))
            }
        } else if file_type.is_socket() {
            ("so", Some('='))
        } else if file_type.is_fifo() {
            ("pi", Some('|'))
        } else if file_type.is_block_device() {
            ("bd", None)
        } else if file_type.is_char_device() {
            ("cd", None)
        } else if file_type.is_file() {
            let mode = metadata.mode() as mode_t;
            let sym = if has!(mode, S_IXUSR | S_IXGRP | S_IXOTH) {
                Some('*')
            } else {
                None
            };
            if has!(mode, S_ISUID) {
                ("su", sym)
            } else if has!(mode, S_ISGID) {
                ("sg", sym)
            } else if has!(mode, S_ISVTX) && has!(mode, S_IWOTH) {
                ("tw", sym)
            } else if has!(mode, S_ISVTX) {
                ("st", sym)
            } else if has!(mode, S_IWOTH) {
                ("ow", sym)
            } else if has!(mode, S_IXUSR | S_IXGRP | S_IXOTH) {
                ("ex", sym)
            } else if metadata.nlink() > 1 {
                ("mh", sym)
            } else if let Some(e) = path.extension() {
                ext = format!("*.{}", e.to_string_lossy());
                (ext.as_str(), None)
            } else {
                ("fi", None)
            }
        } else {
            ("", None)
        };

        if color {
            name = color_name(name, code);
        }
        if classify {
            if let Some(s) = sym {
                name.push(s);
                width += 1;
            }
        }
    }

    if options.opt_present("long") && metadata.file_type().is_symlink() {
        if let Ok(target) = path.read_link() {
            // We don't bother updating width here because it's not used for long listings
            let code = if target.exists() {
                "fi"
            } else {
                "mi"
            };
            let target_name = color_name(target.to_string_lossy().to_string(), code);
            name.push_str(" -> ");
            name.push_str(&target_name);
        }
    }

    Cell {
        contents: name,
        width: width,
    }
}

#[cfg(not(unix))]
#[allow(unused_variables)]
fn display_symlink_count(metadata: &Metadata) -> String {
    // Currently not sure of how to get this on Windows, so I'm punting.
    // Git Bash looks like it may do the same thing.
    String::from("1")
}

#[cfg(unix)]
fn display_symlink_count(metadata: &Metadata) -> String {
    metadata.nlink().to_string()
}

#[cfg(not(unix))]
#[allow(unused_variables)]
fn display_permissions(metadata: &Metadata) -> String {
    String::from("---------")
}

#[cfg(unix)]
fn display_permissions(metadata: &Metadata) -> String {
    let mode = metadata.mode() as mode_t;
    let mut result = String::with_capacity(9);
    result.push(if has!(mode, S_IRUSR) {
        'r'
    } else {
        '-'
    });
    result.push(if has!(mode, S_IWUSR) {
        'w'
    } else {
        '-'
    });
    result.push(if has!(mode, S_ISUID) {
        if has!(mode, S_IXUSR) {
            's'
        } else {
            'S'
        }
    } else if has!(mode, S_IXUSR) {
        'x'
    } else {
        '-'
    });

    result.push(if has!(mode, S_IRGRP) {
        'r'
    } else {
        '-'
    });
    result.push(if has!(mode, S_IWGRP) {
        'w'
    } else {
        '-'
    });
    result.push(if has!(mode, S_ISGID) {
        if has!(mode, S_IXGRP) {
            's'
        } else {
            'S'
        }
    } else if has!(mode, S_IXGRP) {
        'x'
    } else {
        '-'
    });

    result.push(if has!(mode, S_IROTH) {
        'r'
    } else {
        '-'
    });
    result.push(if has!(mode, S_IWOTH) {
        'w'
    } else {
        '-'
    });
    result.push(if has!(mode, S_ISVTX) {
        if has!(mode, S_IXOTH) {
            't'
        } else {
            'T'
        }
    } else if has!(mode, S_IXOTH) {
        'x'
    } else {
        '-'
    });

    result
}
