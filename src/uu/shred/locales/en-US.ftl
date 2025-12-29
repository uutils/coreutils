shred-about = Overwrite the specified FILE(s) repeatedly, in order to make it harder for even
  very expensive hardware probing to recover the data.
shred-usage = shred [OPTION]... FILE...
shred-after-help = Delete FILE(s) if --remove (-u) is specified. The default is not to remove
  the files because it is common to operate on device files like /dev/hda, and
  those files usually should not be removed.

  CAUTION: Note that shred relies on a very important assumption: that the file
  system overwrites data in place. This is the traditional way to do things, but
  many modern file system designs do not satisfy this assumption. The following
  are examples of file systems on which shred is not effective, or is not
  guaranteed to be effective in all file system modes:

   - log-structured or journal file systems, such as those supplied with
     AIX and Solaris (and JFS, ReiserFS, XFS, Ext3, etc.)

   - file systems that write redundant data and carry on even if some writes
     fail, such as RAID-based file systems

   - file systems that make snapshots, such as Network Appliance's NFS server

   - file systems that cache in temporary locations, such as NFS
     version 3 clients

   - compressed file systems

  In the case of ext3 file systems, the above disclaimer applies (and shred is
  thus of limited effectiveness) only in data=journal mode, which journals file
  data in addition to just metadata. In both the data=ordered (default) and
  data=writeback modes, shred works as usual. Ext3 journal modes can be changed
  by adding the data=something option to the mount options for a particular
  file system in the /etc/fstab file, as documented in the mount man page (`man
  mount`).

  In addition, file system backups and remote mirrors may contain copies of
  the file that cannot be removed, and that will allow a shredded file to be
  recovered later.

# Error messages
shred-missing-file-operand = missing file operand
shred-invalid-number-of-passes = invalid number of passes: {$passes}
shred-cannot-open-random-source = cannot open random source: {$source}
shred-invalid-file-size = invalid file size: {$size}
shred-no-such-file-or-directory = {$file}: No such file or directory
shred-not-a-file = {$file}: Not a file

# Option help text
shred-force-help = change permissions to allow writing if necessary
shred-iterations-help = overwrite N times instead of the default (3)
shred-size-help = shred this many bytes (suffixes like K, M, G accepted)
shred-deallocate-help = deallocate and remove file after overwriting
shred-remove-help = like -u but give control on HOW to delete;  See below
shred-verbose-help = show progress
shred-exact-help = do not round file sizes up to the next full block;
                   this is the default for non-regular files
shred-zero-help = add a final overwrite with zeros to hide shredding
shred-random-source-help = take random bytes from FILE

# Verbose messages
shred-removing = {$file}: removing
shred-removed = {$file}: removed
shred-renamed-to = renamed to
shred-pass-progress = {$file}: pass
shred-couldnt-rename = {$file}: Couldn't rename to {$new_name}: {$error}
shred-failed-to-open-for-writing = {$file}: failed to open for writing
shred-file-write-pass-failed = {$file}: File write pass failed
shred-failed-to-remove-file = {$file}: failed to remove file

# File I/O error messages
shred-failed-to-clone-file-handle = failed to clone file handle
shred-failed-to-seek-file = failed to seek in file
shred-failed-to-read-seed-bytes = failed to read seed bytes from file
shred-failed-to-get-metadata = failed to get file metadata
shred-failed-to-set-permissions = failed to set file permissions
