complete -c uu_rm -l interactive -d 'prompt according to WHEN: never, once (-I), or always (-i). Without WHEN, prompts always' -r
complete -c uu_rm -s f -l force -d 'ignore nonexistent files and arguments, never prompt'
complete -c uu_rm -s i -d 'prompt before every removal'
complete -c uu_rm -s I -d 'prompt once before removing more than three files, or when removing recursively. Less intrusive than -i, while still giving some protection against most mistakes'
complete -c uu_rm -l one-file-system -d 'when removing a hierarchy recursively, skip any directory that is on a file system different from that of the corresponding command line argument (NOT IMPLEMENTED)'
complete -c uu_rm -l no-preserve-root -d 'do not treat \'/\' specially'
complete -c uu_rm -l preserve-root -d 'do not remove \'/\' (default)'
complete -c uu_rm -s r -s R -l recursive -d 'remove directories and their contents recursively'
complete -c uu_rm -s d -l dir -d 'remove empty directories'
complete -c uu_rm -s v -l verbose -d 'explain what is being done'
complete -c uu_rm -l presume-input-tty
complete -c uu_rm -s h -l help -d 'Print help'
complete -c uu_rm -s V -l version -d 'Print version'
