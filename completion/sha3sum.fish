complete -c sha3sum -l bits -d 'set the size of the output (only for SHAKE)' -r
complete -c sha3sum -s b -l binary -d 'read in binary mode'
complete -c sha3sum -s c -l check -d 'read hashsums from the FILEs and check them'
complete -c sha3sum -l tag -d 'create a BSD-style checksum'
complete -c sha3sum -s t -l text -d 'read in text mode (default)'
complete -c sha3sum -s q -l quiet -d 'don\'t print OK for each successfully verified file'
complete -c sha3sum -s s -l status -d 'don\'t output anything, status code shows success'
complete -c sha3sum -l strict -d 'exit non-zero for improperly formatted checksum lines'
complete -c sha3sum -l ignore-missing -d 'don\'t fail or report status for missing files'
complete -c sha3sum -s w -l warn -d 'warn about improperly formatted checksum lines'
complete -c sha3sum -s z -l zero -d 'end each output line with NUL, not newline'
complete -c sha3sum -s h -l help -d 'Print help'
complete -c sha3sum -s V -l version -d 'Print version'
