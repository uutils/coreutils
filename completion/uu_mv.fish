complete -c uu_mv -l backup -d 'make a backup of each existing destination file' -r
complete -c uu_mv -s S -l suffix -d 'override the usual backup suffix' -r
complete -c uu_mv -l update -d 'move only when the SOURCE file is newer than the destination file or when the destination file is missing' -r -f -a "{none	,all	,older	}"
complete -c uu_mv -s t -l target-directory -d 'move all SOURCE arguments into DIRECTORY' -r -f -a "(__fish_complete_directories)"
complete -c uu_mv -s f -l force -d 'do not prompt before overwriting'
complete -c uu_mv -s i -l interactive -d 'prompt before override'
complete -c uu_mv -s n -l no-clobber -d 'do not overwrite an existing file'
complete -c uu_mv -l strip-trailing-slashes -d 'remove any trailing slashes from each SOURCE argument'
complete -c uu_mv -s b -d 'like --backup but does not accept an argument'
complete -c uu_mv -s u -d 'like --update but does not accept an argument'
complete -c uu_mv -s T -l no-target-directory -d 'treat DEST as a normal file'
complete -c uu_mv -s v -l verbose -d 'explain what is being done'
complete -c uu_mv -s g -l progress -d 'Display a progress bar. 
Note: this feature is not supported by GNU coreutils.'
complete -c uu_mv -s h -l help -d 'Print help'
complete -c uu_mv -s V -l version -d 'Print version'
