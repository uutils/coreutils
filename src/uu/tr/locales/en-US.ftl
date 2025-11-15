tr-about = Translate or delete characters
tr-usage = tr [OPTION]... SET1 [SET2]
tr-after-help = Translate, squeeze, and/or delete characters from standard input, writing to standard output.

# Help messages
tr-help-complement = use the complement of SET1
tr-help-delete = delete characters in SET1, do not translate
tr-help-squeeze = replace each sequence of a repeated character that is listed in the last specified SET, with a single occurrence of that character
tr-help-truncate-set1 = first truncate SET1 to length of SET2

# Error messages
tr-error-missing-operand = missing operand
tr-error-missing-operand-translating = missing operand after { $set }
  Two strings must be given when translating.
tr-error-missing-operand-deleting-squeezing = missing operand after { $set }
  Two strings must be given when deleting and squeezing.
tr-error-extra-operand-deleting-without-squeezing = extra operand { $operand }
  Only one string may be given when deleting without squeezing repeats.
tr-error-extra-operand-simple = extra operand { $operand }
tr-error-read-directory = read error: Is a directory
tr-error-write-error = write error

# Warning messages
tr-warning-unescaped-backslash = warning: an unescaped backslash at end of string is not portable
tr-warning-ambiguous-octal-escape = the ambiguous octal escape \{ $origin_octal } is being interpreted as the 2-byte sequence \0{ $actual_octal_tail }, { $outstand_char }

# Sequence parsing error messages
tr-error-missing-char-class-name = missing character class name '[::]'
tr-error-missing-equivalence-class-char = missing equivalence class character '[==]'
tr-error-multiple-char-repeat-in-set2 = only one [c*] repeat construct may appear in string2
tr-error-char-repeat-in-set1 = the [c*] repeat construct may not appear in string1
tr-error-invalid-repeat-count = invalid repeat count { $count } in [c*n] construct
tr-error-empty-set2-when-not-truncating = when not truncating set1, string2 must be non-empty
tr-error-class-except-lower-upper-in-set2 = when translating, the only character classes that may appear in set2 are 'upper' and 'lower'
tr-error-class-in-set2-not-matched = when translating, every 'upper'/'lower' in set2 must be matched by a 'upper'/'lower' in the same position in set1
tr-error-set1-longer-set2-ends-in-class = when translating with string1 longer than string2,
  the latter string must not end with a character class
tr-error-complement-more-than-one-unique = when translating with complemented character classes,
  string2 must map all characters in the domain to one
tr-error-backwards-range = range-endpoints of '{ $start }-{ $end }' are in reverse collating sequence order
tr-error-multiple-char-in-equivalence = { $chars }: equivalence class operand must be a single character
