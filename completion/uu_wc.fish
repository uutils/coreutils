complete -c uu_wc -l files0-from -d 'read input from the files specified by
  NUL-terminated names in file F;
  If F is - then read names from standard input' -r -F
complete -c uu_wc -l total -d 'when to print a line with total counts;
  WHEN can be: auto, always, only, never' -r -f -a "{auto	,always	,only	,never	}"
complete -c uu_wc -s c -l bytes -d 'print the byte counts'
complete -c uu_wc -s m -l chars -d 'print the character counts'
complete -c uu_wc -s l -l lines -d 'print the newline counts'
complete -c uu_wc -s L -l max-line-length -d 'print the length of the longest line'
complete -c uu_wc -s w -l words -d 'print the word counts'
complete -c uu_wc -s h -l help -d 'Print help'
complete -c uu_wc -s V -l version -d 'Print version'
