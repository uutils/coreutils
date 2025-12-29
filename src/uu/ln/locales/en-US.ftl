ln-about = Make links between files.
ln-usage = ln [OPTION]... [-T] TARGET LINK_NAME
  ln [OPTION]... TARGET
  ln [OPTION]... TARGET... DIRECTORY
  ln [OPTION]... -t DIRECTORY TARGET...
ln-after-help = In the 1st form, create a link to TARGET with the name LINK_NAME.
  In the 2nd form, create a link to TARGET in the current directory.
  In the 3rd and 4th forms, create links to each TARGET in DIRECTORY.
  Create hard links by default, symbolic links with --symbolic.
  By default, each destination (name of new link) should not already exist.
  When creating hard links, each TARGET must exist. Symbolic links
  can hold arbitrary text; if later resolved, a relative link is
  interpreted in relation to its parent directory.

ln-help-force = remove existing destination files
ln-help-interactive = prompt whether to remove existing destination files
ln-help-no-dereference = treat LINK_NAME as a normal file if it is a
                          symbolic link to a directory
ln-help-logical = follow TARGETs that are symbolic links
ln-help-physical = make hard links directly to symbolic links
ln-help-symbolic = make symbolic links instead of hard links
ln-help-target-directory = specify the DIRECTORY in which to create the links
ln-help-no-target-directory = treat LINK_NAME as a normal file always
ln-help-relative = create symbolic links relative to link location
ln-help-verbose = print name of each linked file
ln-error-target-is-not-directory = target {$target} is not a directory
ln-error-same-file = {$file1} and {$file2} are the same file
ln-error-missing-destination = missing destination file operand after {$operand}
ln-error-extra-operand = extra operand {$operand}
  Try '{$program} --help' for more information.
ln-error-could-not-update = Could not update {$target}: {$error}
ln-error-cannot-stat = cannot stat {$path}: No such file or directory
ln-error-will-not-overwrite = will not overwrite just-created {$target} with {$source}
ln-prompt-replace = replace {$file}?
ln-cannot-backup = cannot backup {$file}
ln-failed-to-access = failed to access {$file}
ln-failed-to-create-hard-link = failed to create hard link {$source} => {$dest}
ln-failed-to-create-hard-link-dir = {$source}: hard link not allowed for directory
ln-backup = backup: {$backup}
