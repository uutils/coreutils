complete -c uu_cp -s t -l target-directory -d 'copy all SOURCE arguments into target-directory' -r -f -a "(__fish_complete_directories)"
complete -c uu_cp -l backup -d 'make a backup of each existing destination file' -r
complete -c uu_cp -s S -l suffix -d 'override the usual backup suffix' -r
complete -c uu_cp -l update -d 'move only when the SOURCE file is newer than the destination file or when the destination file is missing' -r -f -a "{none	,all	,older	}"
complete -c uu_cp -l reflink -d 'control clone/CoW copies. See below' -r -f -a "{auto	,always	,never	}"
complete -c uu_cp -l preserve -d 'Preserve the specified attributes (default: mode, ownership (unix only), timestamps), if possible additional attributes: context, links, xattr, all' -r -f -a "{mode	,ownership	,timestamps	,context	,links	,xattr	,all	}"
complete -c uu_cp -l no-preserve -d 'don\'t preserve the specified attributes' -r -f -a "{mode	,ownership	,timestamps	,context	,links	,xattr	,all	}"
complete -c uu_cp -l sparse -d 'control creation of sparse files. See below' -r -f -a "{never	,auto	,always	}"
complete -c uu_cp -l context -d 'NotImplemented: set SELinux security context of destination file to default type' -r
complete -c uu_cp -s T -l no-target-directory -d 'Treat DEST as a regular file and not a directory'
complete -c uu_cp -s i -l interactive -d 'ask before overwriting files'
complete -c uu_cp -s l -l link -d 'hard-link files instead of copying'
complete -c uu_cp -s n -l no-clobber -d 'don\'t overwrite a file that already exists'
complete -c uu_cp -s R -s r -l recursive -d 'copy directories recursively'
complete -c uu_cp -l strip-trailing-slashes -d 'remove any trailing slashes from each SOURCE argument'
complete -c uu_cp -l debug -d 'explain how a file is copied. Implies -v'
complete -c uu_cp -s v -l verbose -d 'explicitly state what is being done'
complete -c uu_cp -s s -l symbolic-link -d 'make symbolic links instead of copying'
complete -c uu_cp -s f -l force -d 'if an existing destination file cannot be opened, remove it and try again (this option is ignored when the -n option is also used). Currently not implemented for Windows.'
complete -c uu_cp -l remove-destination -d 'remove each existing destination file before attempting to open it (contrast with --force). On Windows, currently only works for writeable files.'
complete -c uu_cp -s b -d 'like --backup but does not accept an argument'
complete -c uu_cp -s u -d 'like --update but does not accept an argument'
complete -c uu_cp -l attributes-only -d 'Don\'t copy the file data, just the attributes'
complete -c uu_cp -s p -l preserve-default-attributes -d 'same as --preserve=mode,ownership(unix only),timestamps'
complete -c uu_cp -l parents -d 'use full source file name under DIRECTORY'
complete -c uu_cp -s P -l no-dereference -d 'never follow symbolic links in SOURCE'
complete -c uu_cp -s L -l dereference -d 'always follow symbolic links in SOURCE'
complete -c uu_cp -s H -d 'follow command-line symbolic links in SOURCE'
complete -c uu_cp -s a -l archive -d 'Same as -dR --preserve=all'
complete -c uu_cp -s d -d 'same as --no-dereference --preserve=links'
complete -c uu_cp -s x -l one-file-system -d 'stay on this file system'
complete -c uu_cp -l copy-contents -d 'NotImplemented: copy contents of special files when recursive'
complete -c uu_cp -s g -l progress -d 'Display a progress bar. 
Note: this feature is not supported by GNU coreutils.'
complete -c uu_cp -s h -l help -d 'Print help'
complete -c uu_cp -s V -l version -d 'Print version'
