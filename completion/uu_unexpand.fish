complete -c uu_unexpand -s t -l tabs -d 'use comma separated LIST of tab positions or have tabs N characters apart instead of 8 (enables -a)' -r
complete -c uu_unexpand -s a -l all -d 'convert all blanks, instead of just initial blanks'
complete -c uu_unexpand -l first-only -d 'convert only leading sequences of blanks (overrides -a)'
complete -c uu_unexpand -s U -l no-utf8 -d 'interpret input file as 8-bit ASCII rather than UTF-8'
complete -c uu_unexpand -s h -l help -d 'Print help'
complete -c uu_unexpand -s V -l version -d 'Print version'
