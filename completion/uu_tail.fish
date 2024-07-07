complete -c uu_tail -s c -l bytes -d 'Number of bytes to print' -r
complete -c uu_tail -s f -l follow -d 'Print the file as it grows' -r -f -a "{descriptor	,name	}"
complete -c uu_tail -s n -l lines -d 'Number of lines to print' -r
complete -c uu_tail -l pid -d 'With -f, terminate after process ID, PID dies' -r
complete -c uu_tail -s s -l sleep-interval -d 'Number of seconds to sleep between polling the file when running with -f' -r
complete -c uu_tail -l max-unchanged-stats -d 'Reopen a FILE which has not changed size after N (default 5) iterations to see if it has been unlinked or renamed (this is the usual case of rotated log files); This option is meaningful only when polling (i.e., with --use-polling) and when --follow=name' -r
complete -c uu_tail -s q -l quiet -l silent -d 'Never output headers giving file names'
complete -c uu_tail -s v -l verbose -d 'Always output headers giving file names'
complete -c uu_tail -s z -l zero-terminated -d 'Line delimiter is NUL, not newline'
complete -c uu_tail -l use-polling -d 'Disable \'inotify\' support and use polling instead'
complete -c uu_tail -l retry -d 'Keep trying to open a file if it is inaccessible'
complete -c uu_tail -s F -d 'Same as --follow=name --retry'
complete -c uu_tail -l presume-input-pipe
complete -c uu_tail -s h -l help -d 'Print help'
complete -c uu_tail -s V -l version -d 'Print version'
