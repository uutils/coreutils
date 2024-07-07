complete -c uu_ptx -s F -l flag-truncation -d 'use STRING for flagging line truncations' -r
complete -c uu_ptx -s M -l macro-name -d 'macro name to use instead of \'xx\'' -r
complete -c uu_ptx -s S -l sentence-regexp -d 'for end of lines or end of sentences' -r
complete -c uu_ptx -s W -l word-regexp -d 'use REGEXP to match each keyword' -r
complete -c uu_ptx -s b -l break-file -d 'word break characters in this FILE' -r -F
complete -c uu_ptx -s g -l gap-size -d 'gap size in columns between output fields' -r
complete -c uu_ptx -s i -l ignore-file -d 'read ignore word list from FILE' -r -F
complete -c uu_ptx -s o -l only-file -d 'read only word list from this FILE' -r -F
complete -c uu_ptx -s w -l width -d 'output width in columns, reference excluded' -r
complete -c uu_ptx -s A -l auto-reference -d 'output automatically generated references'
complete -c uu_ptx -s G -l traditional -d 'behave more like System V \'ptx\''
complete -c uu_ptx -s O -l format=roff -d 'generate output as roff directives'
complete -c uu_ptx -s R -l right-side-refs -d 'put references at right, not counted in -w'
complete -c uu_ptx -s T -l format=tex -d 'generate output as TeX directives'
complete -c uu_ptx -s f -l ignore-case -d 'fold lower case to upper case for sorting'
complete -c uu_ptx -s r -l references -d 'first field of each line is a reference'
complete -c uu_ptx -s h -l help -d 'Print help'
complete -c uu_ptx -s V -l version -d 'Print version'
