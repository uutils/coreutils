complete -c uu_numfmt -s d -l delimiter -d 'use X instead of whitespace for field delimiter' -r
complete -c uu_numfmt -l field -d 'replace the numbers in these input fields; see FIELDS below' -r
complete -c uu_numfmt -l format -d 'use printf style floating-point FORMAT; see FORMAT below for details' -r
complete -c uu_numfmt -l from -d 'auto-scale input numbers to UNITs; see UNIT below' -r
complete -c uu_numfmt -l from-unit -d 'specify the input unit size' -r
complete -c uu_numfmt -l to -d 'auto-scale output numbers to UNITs; see UNIT below' -r
complete -c uu_numfmt -l to-unit -d 'the output unit size' -r
complete -c uu_numfmt -l padding -d 'pad the output to N characters; positive N will right-align; negative N will left-align; padding is ignored if the output is wider than N; the default is to automatically pad if a whitespace is found' -r
complete -c uu_numfmt -l header -d 'print (without converting) the first N header lines; N defaults to 1 if not specified' -r
complete -c uu_numfmt -l round -d 'use METHOD for rounding when scaling' -r -f -a "{up	,down	,from-zero	,towards-zero	,nearest	}"
complete -c uu_numfmt -l suffix -d 'print SUFFIX after each formatted number, and accept inputs optionally ending with SUFFIX' -r
complete -c uu_numfmt -l invalid -d 'set the failure mode for invalid input' -r -f -a "{abort	,fail	,warn	,ignore	}"
complete -c uu_numfmt -s h -l help -d 'Print help'
complete -c uu_numfmt -s V -l version -d 'Print version'
