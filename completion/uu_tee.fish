complete -c uu_tee -l output-error -d 'set write error behavior' -r -f -a "{warn	produce warnings for errors writing to any output,warn-nopipe	produce warnings for errors that are not pipe errors (ignored on non-unix platforms),exit	exit on write errors to any output,exit-nopipe	exit on write errors to any output that are not pipe errors (equivalent to exit on non-unix platforms)}"
complete -c uu_tee -s h -l help -d 'Print help'
complete -c uu_tee -s a -l append -d 'append to the given FILEs, do not overwrite'
complete -c uu_tee -s i -l ignore-interrupts -d 'ignore interrupt signals (ignored on non-Unix platforms)'
complete -c uu_tee -s p -d 'set write error behavior (ignored on non-Unix platforms)'
complete -c uu_tee -s V -l version -d 'Print version'
