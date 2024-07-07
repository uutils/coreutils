complete -c uu_install -l backup -d 'make a backup of each existing destination file' -r
complete -c uu_install -s g -l group -d 'set group ownership, instead of process\'s current group' -r
complete -c uu_install -s m -l mode -d 'set permission mode (as in chmod), instead of rwxr-xr-x' -r
complete -c uu_install -s o -l owner -d 'set ownership (super-user only)' -r -f -a "(__fish_complete_users)"
complete -c uu_install -l strip-program -d 'program used to strip binaries (no action Windows)' -r -f -a "(__fish_complete_command)"
complete -c uu_install -s S -l suffix -d 'override the usual backup suffix' -r
complete -c uu_install -s t -l target-directory -d 'move all SOURCE arguments into DIRECTORY' -r -f -a "(__fish_complete_directories)"
complete -c uu_install -s b -d 'like --backup but does not accept an argument'
complete -c uu_install -s c -d 'ignored'
complete -c uu_install -s C -l compare -d 'compare each pair of source and destination files, and in some cases, do not modify the destination at all'
complete -c uu_install -s d -l directory -d 'treat all arguments as directory names. create all components of the specified directories'
complete -c uu_install -s D -d 'create all leading components of DEST except the last, then copy SOURCE to DEST'
complete -c uu_install -s p -l preserve-timestamps -d 'apply access/modification times of SOURCE files to corresponding destination files'
complete -c uu_install -s s -l strip -d 'strip symbol tables (no action Windows)'
complete -c uu_install -s T -l no-target-directory -d '(unimplemented) treat DEST as a normal file'
complete -c uu_install -s v -l verbose -d 'explain what is being done'
complete -c uu_install -s P -l preserve-context -d '(unimplemented) preserve security context'
complete -c uu_install -s Z -l context -d '(unimplemented) set security context of files and directories'
complete -c uu_install -s h -l help -d 'Print help'
complete -c uu_install -s V -l version -d 'Print version'
