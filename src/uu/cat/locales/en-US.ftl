cat-about = Concatenate FILE(s), or standard input, to standard output
  With no FILE, or when FILE is -, read standard input.
cat-usage = cat [OPTION]... [FILE]...

# Help messages
cat-help-show-all = equivalent to -vET
cat-help-number-nonblank = number nonempty output lines, overrides -n
cat-help-show-nonprinting-ends = equivalent to -vE
cat-help-show-ends = display $ at end of each line
cat-help-number = number all output lines
cat-help-squeeze-blank = suppress repeated empty output lines
cat-help-show-nonprinting-tabs = equivalent to -vT
cat-help-show-tabs = display TAB characters at ^I
cat-help-show-nonprinting = use ^ and M- notation, except for LF (\n) and TAB (\t)
cat-help-ignored-u = (ignored)

# Error messages
cat-error-unknown-filetype = unknown filetype: { $ft_debug }
cat-error-is-directory = Is a directory
cat-error-input-file-is-output-file = input file is output file
cat-error-too-many-symbolic-links = Too many levels of symbolic links
cat-error-no-such-device-or-address = No such device or address
