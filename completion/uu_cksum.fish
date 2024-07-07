complete -c uu_cksum -s a -l algorithm -d 'select the digest type to use. See DIGEST below' -r -f -a "{sysv	,bsd	,crc	,md5	,sha1	,sha3	,sha224	,sha256	,sha384	,sha512	,blake2b	,blake3	,sm3	,shake128	,shake256	}"
complete -c uu_cksum -s l -l length -d 'digest length in bits; must not exceed the max for the blake2 algorithm and must be a multiple of 8' -r
complete -c uu_cksum -l untagged -d 'create a reversed style checksum, without digest type'
complete -c uu_cksum -l tag -d 'create a BSD style checksum, undo --untagged (default)'
complete -c uu_cksum -l raw -d 'emit a raw binary digest, not hexadecimal'
complete -c uu_cksum -l strict -d 'exit non-zero for improperly formatted checksum lines'
complete -c uu_cksum -s c -l check -d 'read hashsums from the FILEs and check them'
complete -c uu_cksum -l base64 -d 'emit a base64 digest, not hexadecimal'
complete -c uu_cksum -s t -l text
complete -c uu_cksum -s b -l binary
complete -c uu_cksum -s w -l warn -d 'warn about improperly formatted checksum lines'
complete -c uu_cksum -l status -d 'don\'t output anything, status code shows success'
complete -c uu_cksum -l quiet -d 'don\'t print OK for each successfully verified file'
complete -c uu_cksum -l ignore-missing -d 'don\'t fail or report status for missing files'
complete -c uu_cksum -s h -l help -d 'Print help'
complete -c uu_cksum -s V -l version -d 'Print version'
