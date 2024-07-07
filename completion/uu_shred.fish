complete -c uu_shred -s n -l iterations -d 'overwrite N times instead of the default (3)' -r
complete -c uu_shred -s s -l size -d 'shred this many bytes (suffixes like K, M, G accepted)' -r
complete -c uu_shred -l remove -d 'like -u but give control on HOW to delete;  See below' -r -f -a "{unlink	,wipe	,wipesync	}"
complete -c uu_shred -s f -l force -d 'change permissions to allow writing if necessary'
complete -c uu_shred -s u -d 'deallocate and remove file after overwriting'
complete -c uu_shred -s v -l verbose -d 'show progress'
complete -c uu_shred -s x -l exact -d 'do not round file sizes up to the next full block;
this is the default for non-regular files'
complete -c uu_shred -s z -l zero -d 'add a final overwrite with zeros to hide shredding'
complete -c uu_shred -s h -l help -d 'Print help'
complete -c uu_shred -s V -l version -d 'Print version'
