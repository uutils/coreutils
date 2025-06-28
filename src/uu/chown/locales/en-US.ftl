chown-about = Change file owner and group
chown-usage = chown [OPTION]... [OWNER][:[GROUP]] FILE...
  chown [OPTION]... --reference=RFILE FILE...

# Help messages
chown-help-print-help = Print help information.
chown-help-changes = like verbose but report only when a change is made
chown-help-from = change the owner and/or group of each file only if its
  current owner and/or group match those specified here.
  Either may be omitted, in which case a match is not required
  for the omitted attribute
chown-help-preserve-root = fail to operate recursively on '/'
chown-help-no-preserve-root = do not treat '/' specially (the default)
chown-help-quiet = suppress most error messages
chown-help-recursive = operate on files and directories recursively
chown-help-reference = use RFILE's owner and group rather than specifying OWNER:GROUP values
chown-help-verbose = output a diagnostic for every file processed

# Error messages
chown-error-failed-to-get-attributes = failed to get attributes of { $file }
chown-error-invalid-user = invalid user: { $user }
chown-error-invalid-group = invalid group: { $group }
chown-error-invalid-spec = invalid spec: { $spec }
