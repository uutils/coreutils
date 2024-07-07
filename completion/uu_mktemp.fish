complete -c uu_mktemp -l suffix -d 'append SUFFIX to TEMPLATE; SUFFIX must not contain a path separator. This option is implied if TEMPLATE does not end with X.' -r
complete -c uu_mktemp -s p -d 'short form of --tmpdir' -r -f -a "(__fish_complete_directories)"
complete -c uu_mktemp -l tmpdir -d 'interpret TEMPLATE relative to DIR; if DIR is not specified, use $TMPDIR ($TMP on windows) if set, else /tmp. With this option, TEMPLATE must not be an absolute name; unlike with -t, TEMPLATE may contain slashes, but mktemp creates only the final component' -r -f -a "(__fish_complete_directories)"
complete -c uu_mktemp -s d -l directory -d 'Make a directory instead of a file'
complete -c uu_mktemp -s u -l dry-run -d 'do not create anything; merely print a name (unsafe)'
complete -c uu_mktemp -s q -l quiet -d 'Fail silently if an error occurs.'
complete -c uu_mktemp -s t -d 'Generate a template (using the supplied prefix and TMPDIR (TMP on windows) if set) to create a filename template [deprecated]'
complete -c uu_mktemp -s h -l help -d 'Print help'
complete -c uu_mktemp -s V -l version -d 'Print version'
