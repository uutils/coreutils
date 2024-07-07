complete -c uu_ln -l backup -d 'make a backup of each existing destination file' -r
complete -c uu_ln -s S -l suffix -d 'override the usual backup suffix' -r
complete -c uu_ln -s t -l target-directory -d 'specify the DIRECTORY in which to create the links' -r -f -a "(__fish_complete_directories)"
complete -c uu_ln -s b -d 'like --backup but does not accept an argument'
complete -c uu_ln -s f -l force -d 'remove existing destination files'
complete -c uu_ln -s i -l interactive -d 'prompt whether to remove existing destination files'
complete -c uu_ln -s n -l no-dereference -d 'treat LINK_NAME as a normal file if it is a symbolic link to a directory'
complete -c uu_ln -s L -l logical -d 'follow TARGETs that are symbolic links'
complete -c uu_ln -s P -l physical -d 'make hard links directly to symbolic links'
complete -c uu_ln -s s -l symbolic -d 'make symbolic links instead of hard links'
complete -c uu_ln -s T -l no-target-directory -d 'treat LINK_NAME as a normal file always'
complete -c uu_ln -s r -l relative -d 'create symbolic links relative to link location'
complete -c uu_ln -s v -l verbose -d 'print name of each linked file'
complete -c uu_ln -s h -l help -d 'Print help'
complete -c uu_ln -s V -l version -d 'Print version'
