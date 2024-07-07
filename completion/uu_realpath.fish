complete -c uu_realpath -l relative-to -d 'print the resolved path relative to DIR' -r
complete -c uu_realpath -l relative-base -d 'print absolute paths unless paths below DIR' -r
complete -c uu_realpath -s q -l quiet -d 'Do not print warnings for invalid paths'
complete -c uu_realpath -s s -l strip -l no-symlinks -d 'Only strip \'.\' and \'..\' components, but don\'t resolve symbolic links'
complete -c uu_realpath -s z -l zero -d 'Separate output filenames with \\0 rather than newline'
complete -c uu_realpath -s L -l logical -d 'resolve \'..\' components before symlinks'
complete -c uu_realpath -s P -l physical -d 'resolve symlinks as encountered (default)'
complete -c uu_realpath -s e -l canonicalize-existing -d 'canonicalize by following every symlink in every component of the given name recursively, all components must exist'
complete -c uu_realpath -s m -l canonicalize-missing -d 'canonicalize by following every symlink in every component of the given name recursively, without requirements on components existence'
complete -c uu_realpath -s h -l help -d 'Print help'
complete -c uu_realpath -s V -l version -d 'Print version'
