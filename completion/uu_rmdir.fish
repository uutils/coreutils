complete -c uu_rmdir -l ignore-fail-on-non-empty -d 'ignore each failure that is solely because a directory is non-empty'
complete -c uu_rmdir -s p -l parents -d 'remove DIRECTORY and its ancestors; e.g.,
                  \'rmdir -p a/b/c\' is similar to rmdir a/b/c a/b a'
complete -c uu_rmdir -s v -l verbose -d 'output a diagnostic for every directory processed'
complete -c uu_rmdir -s h -l help -d 'Print help'
complete -c uu_rmdir -s V -l version -d 'Print version'
