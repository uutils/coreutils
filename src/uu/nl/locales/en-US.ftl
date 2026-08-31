nl-about = Number lines of files
nl-usage = nl [OPTION]... [FILE]...
nl-after-help = STYLE is one of:

    - a number all lines
    - t number only nonempty lines
    - n number no lines
    - pBRE number only lines that contain a match for the basic regular expression, BRE

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
nl-error-could-not-read-line = could not read line
nl-error-could-not-write = could not write output
nl-error-line-number-overflow = line number overflow
nl-error-invalid-regex = Invalid regular expression
nl-error-invalid-numbering-style = invalid { $kind } numbering style: '{ $value }'
nl-error-invalid-number-format = invalid line numbering format: '{ $value }'
nl-error-invalid-number = invalid { $kind }: '{ $value }'
nl-error-number-out-of-range = invalid { $kind }: '{ $value }': Numerical result out of range
nl-error-number-too-large = invalid { $kind }: '{ $value }': Value too large for defined data type
nl-error-try-help = Try 'nl --help' for more information.
nl-error-is-directory = { $path }: Is a directory
