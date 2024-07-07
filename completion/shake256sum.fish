complete -c shake256sum -l bits -d 'set the size of the output (only for SHAKE)' -r
complete -c shake256sum -s b -l binary -d 'read in binary mode'
complete -c shake256sum -s c -l check -d 'read hashsums from the FILEs and check them'
complete -c shake256sum -l tag -d 'create a BSD-style checksum'
complete -c shake256sum -s t -l text -d 'read in text mode (default)'
complete -c shake256sum -s q -l quiet -d 'don\'t print OK for each successfully verified file'
complete -c shake256sum -s s -l status -d 'don\'t output anything, status code shows success'
complete -c shake256sum -l strict -d 'exit non-zero for improperly formatted checksum lines'
complete -c shake256sum -l ignore-missing -d 'don\'t fail or report status for missing files'
complete -c shake256sum -s w -l warn -d 'warn about improperly formatted checksum lines'
complete -c shake256sum -s z -l zero -d 'end each output line with NUL, not newline'
complete -c shake256sum -s h -l help -d 'Print help'
complete -c shake256sum -s V -l version -d 'Print version'
