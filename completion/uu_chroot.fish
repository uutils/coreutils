complete -c uu_chroot -s u -l user -d 'User (ID or name) to switch before running the program' -r
complete -c uu_chroot -s g -l group -d 'Group (ID or name) to switch to' -r
complete -c uu_chroot -s G -l groups -d 'Comma-separated list of groups to switch to' -r
complete -c uu_chroot -l userspec -d 'Colon-separated user and group to switch to. Same as -u USER -g GROUP. Userspec has higher preference than -u and/or -g' -r
complete -c uu_chroot -l skip-chdir -d 'Use this option to not change the working directory to / after changing the root directory to newroot, i.e., inside the chroot.'
complete -c uu_chroot -s h -l help -d 'Print help'
complete -c uu_chroot -s V -l version -d 'Print version'
