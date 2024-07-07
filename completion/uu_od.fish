complete -c uu_od -s A -l address-radix -d 'Select the base in which file offsets are printed.' -r
complete -c uu_od -s j -l skip-bytes -d 'Skip bytes input bytes before formatting and writing.' -r
complete -c uu_od -s N -l read-bytes -d 'limit dump to BYTES input bytes' -r
complete -c uu_od -l endian -d 'byte order to use for multi-byte formats' -r -f -a "{big	,little	}"
complete -c uu_od -s S -l strings -d 'NotImplemented: output strings of at least BYTES graphic chars. 3 is assumed when BYTES is not specified.' -r
complete -c uu_od -s t -l format -d 'select output format or formats' -r
complete -c uu_od -s w -l width -d 'output BYTES bytes per output line. 32 is implied when BYTES is not specified.' -r
complete -c uu_od -l help -d 'Print help information.'
complete -c uu_od -s a -d 'named characters, ignoring high-order bit'
complete -c uu_od -s b -d 'octal bytes'
complete -c uu_od -s c -d 'ASCII characters or backslash escapes'
complete -c uu_od -s d -d 'unsigned decimal 2-byte units'
complete -c uu_od -s D -d 'unsigned decimal 4-byte units'
complete -c uu_od -s o -d 'octal 2-byte units'
complete -c uu_od -s I -d 'decimal 8-byte units'
complete -c uu_od -s L -d 'decimal 8-byte units'
complete -c uu_od -s i -d 'decimal 4-byte units'
complete -c uu_od -s l -d 'decimal 8-byte units'
complete -c uu_od -s x -d 'hexadecimal 2-byte units'
complete -c uu_od -s h -d 'hexadecimal 2-byte units'
complete -c uu_od -s O -d 'octal 4-byte units'
complete -c uu_od -s s -d 'decimal 2-byte units'
complete -c uu_od -s X -d 'hexadecimal 4-byte units'
complete -c uu_od -s H -d 'hexadecimal 4-byte units'
complete -c uu_od -s e -d 'floating point double precision (64-bit) units'
complete -c uu_od -s f -d 'floating point double precision (32-bit) units'
complete -c uu_od -s F -d 'floating point double precision (64-bit) units'
complete -c uu_od -s v -l output-duplicates -d 'do not use * to mark line suppression'
complete -c uu_od -l traditional -d 'compatibility mode with one input, offset and label.'
complete -c uu_od -s V -l version -d 'Print version'
