chroot-about = Run COMMAND with root directory set to NEWROOT.
chroot-usage = chroot [OPTION]... NEWROOT [COMMAND [ARG]...]

# Help messages
chroot-help-groups = Comma-separated list of groups to switch to
chroot-help-userspec = Colon-separated user and group to switch to.
chroot-help-skip-chdir = Use this option to not change the working directory to / after changing the root directory to newroot, i.e., inside the chroot.

# Error messages
chroot-error-skip-chdir-only-permitted = option --skip-chdir only permitted if NEWROOT is old '/'
chroot-error-cannot-enter = cannot chroot to { $dir }: { $err }
chroot-error-command-failed = failed to run command { $cmd }: { $err }
chroot-error-command-not-found = failed to run command { $cmd }: { $err }
chroot-error-groups-parsing-failed = --groups parsing failed
chroot-error-invalid-group = invalid group: { $group }
chroot-error-invalid-group-list = invalid group list: { $list }
chroot-error-missing-newroot = Missing operand: NEWROOT\nTry '{ $util_name } --help' for more information.
chroot-error-no-group-specified = no group specified for unknown uid: { $uid }
chroot-error-no-such-user = invalid user
chroot-error-no-such-group = invalid group
chroot-error-no-such-directory = cannot change root directory to { $dir }: no such directory
chroot-error-set-gid-failed = cannot set gid to { $gid }: { $err }
chroot-error-set-groups-failed = cannot set groups: { $err }
chroot-error-set-user-failed = cannot set user to { $user }: { $err }
