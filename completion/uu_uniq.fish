complete -c uu_uniq -s D -l all-repeated -d 'print all duplicate lines. Delimiting is done with blank lines. [default: none]' -r -f -a "{none	,prepend	,separate	}"
complete -c uu_uniq -l group -d 'show all items, separating groups with an empty line. [default: separate]' -r -f -a "{separate	,prepend	,append	,both	}"
complete -c uu_uniq -s w -l check-chars -d 'compare no more than N characters in lines' -r
complete -c uu_uniq -s s -l skip-chars -d 'avoid comparing the first N characters' -r
complete -c uu_uniq -s f -l skip-fields -d 'avoid comparing the first N fields' -r
complete -c uu_uniq -s c -l count -d 'prefix lines by the number of occurrences'
complete -c uu_uniq -s i -l ignore-case -d 'ignore differences in case when comparing'
complete -c uu_uniq -s d -l repeated -d 'only print duplicate lines'
complete -c uu_uniq -s u -l unique -d 'only print unique lines'
complete -c uu_uniq -s z -l zero-terminated -d 'end lines with 0 byte, not newline'
complete -c uu_uniq -s h -l help -d 'Print help'
complete -c uu_uniq -s V -l version -d 'Print version'
