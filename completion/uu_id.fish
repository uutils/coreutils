complete -c uu_id -s A -d 'Display the process audit user ID and other process audit properties,
which requires privilege (not available on Linux).'
complete -c uu_id -s u -l user -d 'Display only the effective user ID as a number.'
complete -c uu_id -s g -l group -d 'Display only the effective group ID as a number'
complete -c uu_id -s G -l groups -d 'Display only the different group IDs as white-space separated numbers, in no particular order.'
complete -c uu_id -s p -d 'Make the output human-readable. Each display is on a separate line.'
complete -c uu_id -s n -l name -d 'Display the name of the user or group ID for the -G, -g and -u options instead of the number.
If any of the ID numbers cannot be mapped into names, the number will be displayed as usual.'
complete -c uu_id -s P -d 'Display the id as a password file entry.'
complete -c uu_id -s r -l real -d 'Display the real ID for the -G, -g and -u options instead of the effective ID.'
complete -c uu_id -s z -l zero -d 'delimit entries with NUL characters, not whitespace;
not permitted in default format'
complete -c uu_id -s Z -l context -d 'print only the security context of the process (not enabled)'
complete -c uu_id -s h -l help -d 'Print help'
complete -c uu_id -s V -l version -d 'Print version'
