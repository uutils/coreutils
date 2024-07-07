
use builtin;
use str;

set edit:completion:arg-completer[uu_split] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_split'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_split'= {
            cand -b 'put SIZE bytes per output file'
            cand --bytes 'put SIZE bytes per output file'
            cand -C 'put at most SIZE bytes of lines per output file'
            cand --line-bytes 'put at most SIZE bytes of lines per output file'
            cand -l 'put NUMBER lines/records per output file'
            cand --lines 'put NUMBER lines/records per output file'
            cand -n 'generate CHUNKS output files; see explanation below'
            cand --number 'generate CHUNKS output files; see explanation below'
            cand --additional-suffix 'additional SUFFIX to append to output file names'
            cand --filter 'write to shell COMMAND; file name is $FILE (Currently not implemented for Windows)'
            cand --numeric-suffixes 'same as -d, but allow setting the start value'
            cand --hex-suffixes 'same as -x, but allow setting the start value'
            cand -a 'generate suffixes of length N (default 2)'
            cand --suffix-length 'generate suffixes of length N (default 2)'
            cand -t 'use SEP instead of newline as the record separator; ''\0'' (zero) specifies the NUL character'
            cand --separator 'use SEP instead of newline as the record separator; ''\0'' (zero) specifies the NUL character'
            cand --io-blksize 'io-blksize'
            cand -e 'do not generate empty output files with ''-n'''
            cand --elide-empty-files 'do not generate empty output files with ''-n'''
            cand -d 'use numeric suffixes starting at 0, not alphabetic'
            cand -x 'use hex suffixes starting at 0, not alphabetic'
            cand --verbose 'print a diagnostic just before each output file is opened'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
