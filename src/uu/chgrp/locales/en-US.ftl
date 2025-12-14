chgrp-about = Change the group of each FILE to GROUP.
chgrp-usage = chgrp [OPTION]... GROUP FILE...
  chgrp [OPTION]... --reference=RFILE FILE...

# Help messages
chgrp-help-print-help = Print help information.
chgrp-help-changes = like verbose but report only when a change is made
chgrp-help-quiet = suppress most error messages
chgrp-help-verbose = output a diagnostic for every file processed
chgrp-help-preserve-root = fail to operate recursively on '/'
chgrp-help-no-preserve-root = do not treat '/' specially (the default)
chgrp-help-reference = use RFILE's group rather than specifying GROUP values
chgrp-help-from = change the group only if its current group matches GROUP
chgrp-help-recursive = operate on files and directories recursively

# Error messages
chgrp-error-invalid-group-id = invalid group id: '{ $gid_str }'
chgrp-error-invalid-group = invalid group: '{ $group }'
chgrp-error-failed-to-get-attributes = failed to get attributes of { $file }
chgrp-error-invalid-user = invalid user: '{ $from_group }'
