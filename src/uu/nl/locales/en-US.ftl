nl-about = Number lines of files
nl-usage = nl [OPTION]... [FILE]...
nl-after-help = STYLE is one of:

  - a number all lines
  - t number only nonempty lines
  - n number no lines
  - pBRE number only lines that contain a match for the basic regular
          expression, BRE

  FORMAT is one of:

  - ln left justified, no leading zeros
  - rn right justified, no leading zeros
  - rz right justified, leading zeros

# Help messages
nl-help-help = Print help information.
nl-help-body-numbering = use STYLE for numbering body lines
nl-help-section-delimiter = use CC for separating logical pages
nl-help-footer-numbering = use STYLE for numbering footer lines
nl-help-header-numbering = use STYLE for numbering header lines
nl-help-line-increment = line number increment at each line
nl-help-join-blank-lines = group of NUMBER empty lines counted as one
nl-help-number-format = insert line numbers according to FORMAT
nl-help-no-renumber = do not reset line numbers at logical pages
nl-help-number-separator = add STRING after (possible) line number
nl-help-starting-line-number = first line number on each logical page
nl-help-number-width = use NUMBER columns for line numbers

# Error messages
nl-error-invalid-arguments = Invalid arguments supplied.
nl-error-could-not-read-line = could not read line
nl-error-could-not-write = could not write output
nl-error-line-number-overflow = line number overflow
nl-error-invalid-line-width = Invalid line number field width: ‘{ $value }’: Numerical result out of range
nl-error-invalid-regex = invalid regular expression
nl-error-invalid-numbering-style = invalid numbering style: '{ $style }'
nl-error-is-directory = { $path }: Is a directory
