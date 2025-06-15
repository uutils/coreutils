mktemp-about = Create a temporary file or directory.
mktemp-usage = mktemp [OPTION]... [TEMPLATE]

# Help messages
mktemp-help-directory = Make a directory instead of a file
mktemp-help-dry-run = do not create anything; merely print a name (unsafe)
mktemp-help-quiet = Fail silently if an error occurs.
mktemp-help-suffix = append SUFFIX to TEMPLATE; SUFFIX must not contain a path separator. This option is implied if TEMPLATE does not end with X.
mktemp-help-p = short form of --tmpdir
mktemp-help-tmpdir = interpret TEMPLATE relative to DIR; if DIR is not specified, use $TMPDIR ($TMP on windows) if set, else /tmp. With this option, TEMPLATE must not be an absolute name; unlike with -t, TEMPLATE may contain slashes, but mktemp creates only the final component
mktemp-help-t = Generate a template (using the supplied prefix and TMPDIR (TMP on windows) if set) to create a filename template [deprecated]

# Error messages
mktemp-error-persist-file = could not persist file { $path }
mktemp-error-must-end-in-x = with --suffix, template { $template } must end in X
mktemp-error-too-few-xs = too few X's in template { $template }
mktemp-error-prefix-contains-separator = invalid template, { $template }, contains directory separator
mktemp-error-suffix-contains-separator = invalid suffix { $suffix }, contains directory separator
mktemp-error-invalid-template = invalid template, { $template }; with --tmpdir, it may not be absolute
mktemp-error-too-many-templates = too many templates
mktemp-error-not-found = failed to create { $template_type } via template { $template }: No such file or directory
mktemp-error-failed-print = failed to print directory name

# Template types
mktemp-template-type-directory = directory
mktemp-template-type-file = file
