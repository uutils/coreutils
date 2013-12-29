fn paths_refer_to_the_same_file(p1: &Path, p2: &Path) -> bool {
    let mut raw_p1 = p1.clone();
    let mut raw_p2 = p2.clone();

    // We have to take symlinks and relative paths into account.
    if fs::lstat(raw_p1).kind == io::TypeSymlink {
        raw_p1 = ~fs::readlink(raw_p1).unwrap();
    }
    raw_p1 = ~os::make_absolute(raw_p1);
    if fs::lstat(raw_p2).kind == io::TypeSymlink {
        raw_p2 = ~fs::readlink(raw_p2).unwrap();
    }
    raw_p2 = ~os::make_absolute(raw_p2);

    raw_p1 == raw_p2
}
