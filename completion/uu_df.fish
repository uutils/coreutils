complete -c uu_df -s B -l block-size -d 'scale sizes by SIZE before printing them; e.g.\'-BM\' prints sizes in units of 1,048,576 bytes' -r
complete -c uu_df -l output -d 'use the output format defined by FIELD_LIST, or print all fields if FIELD_LIST is omitted.' -r -f -a "{source	,fstype	,itotal	,iused	,iavail	,ipcent	,size	,used	,avail	,pcent	,file	,target	}"
complete -c uu_df -s t -l type -d 'limit listing to file systems of type TYPE' -r
complete -c uu_df -s x -l exclude-type -d 'limit listing to file systems not of type TYPE' -r
complete -c uu_df -l help -d 'Print help information.'
complete -c uu_df -s a -l all -d 'include dummy file systems'
complete -c uu_df -l total -d 'produce a grand total'
complete -c uu_df -s h -l human-readable -d 'print sizes in human readable format (e.g., 1K 234M 2G)'
complete -c uu_df -s H -l si -d 'likewise, but use powers of 1000 not 1024'
complete -c uu_df -s i -l inodes -d 'list inode information instead of block usage'
complete -c uu_df -s k -d 'like --block-size=1K'
complete -c uu_df -s l -l local -d 'limit listing to local file systems'
complete -c uu_df -l no-sync -d 'do not invoke sync before getting usage info (default)'
complete -c uu_df -s P -l portability -d 'use the POSIX output format'
complete -c uu_df -l sync -d 'invoke sync before getting usage info (non-windows only)'
complete -c uu_df -s T -l print-type -d 'print file system type'
complete -c uu_df -s V -l version -d 'Print version'
