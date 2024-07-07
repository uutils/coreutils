complete -c uu_env -s C -l chdir -d 'change working directory to DIR' -r -f -a "(__fish_complete_directories)"
complete -c uu_env -s f -l file -d 'read and set variables from a ".env"-style configuration file (prior to any unset and/or set)' -r -F
complete -c uu_env -s u -l unset -d 'remove variable from the environment' -r
complete -c uu_env -s S -l split-string -d 'process and split S into separate arguments; used to pass multiple arguments on shebang lines' -r
complete -c uu_env -s a -l argv0 -d 'Override the zeroth argument passed to the command being executed. Without this option a default value of `command` is used.' -r
complete -c uu_env -l ignore-signal -d 'set handling of SIG signal(s) to do nothing' -r
complete -c uu_env -s i -l ignore-environment -d 'start with an empty environment'
complete -c uu_env -s 0 -l null -d 'end each output line with a 0 byte rather than a newline (only valid when printing the environment)'
complete -c uu_env -s v -l debug -d 'print verbose information for each processing step'
complete -c uu_env -s h -l help -d 'Print help'
complete -c uu_env -s V -l version -d 'Print version'
