complete -c uu_basenc -s w -l wrap -d 'wrap encoded lines after COLS character (default 76, 0 to disable wrapping)' -r
complete -c uu_basenc -s d -l decode -d 'decode data'
complete -c uu_basenc -s i -l ignore-garbage -d 'when decoding, ignore non-alphabetic characters'
complete -c uu_basenc -l base64 -d 'same as \'base64\' program'
complete -c uu_basenc -l base64url -d 'file- and url-safe base64'
complete -c uu_basenc -l base32 -d 'same as \'base32\' program'
complete -c uu_basenc -l base32hex -d 'extended hex alphabet base32'
complete -c uu_basenc -l base16 -d 'hex encoding'
complete -c uu_basenc -l base2lsbf -d 'bit string with least significant bit (lsb) first'
complete -c uu_basenc -l base2msbf -d 'bit string with most significant bit (msb) first'
complete -c uu_basenc -l z85 -d 'ascii85-like encoding;
when encoding, input length must be a multiple of 4;
when decoding, input length must be a multiple of 5'
complete -c uu_basenc -s h -l help -d 'Print help'
complete -c uu_basenc -s V -l version -d 'Print version'
