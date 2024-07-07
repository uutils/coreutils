
use builtin;
use str;

set edit:completion:arg-completer[uu_readlink] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_readlink'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_readlink'= {
            cand -f 'canonicalize by following every symlink in every component of the given name recursively; all but the last component must exist'
            cand --canonicalize 'canonicalize by following every symlink in every component of the given name recursively; all but the last component must exist'
            cand -e 'canonicalize by following every symlink in every component of the given name recursively, all components must exist'
            cand --canonicalize-existing 'canonicalize by following every symlink in every component of the given name recursively, all components must exist'
            cand -m 'canonicalize by following every symlink in every component of the given name recursively, without requirements on components existence'
            cand --canonicalize-missing 'canonicalize by following every symlink in every component of the given name recursively, without requirements on components existence'
            cand -n 'do not output the trailing delimiter'
            cand --no-newline 'do not output the trailing delimiter'
            cand -q 'suppress most error messages'
            cand --quiet 'suppress most error messages'
            cand -s 'suppress most error messages'
            cand --silent 'suppress most error messages'
            cand -v 'report error message'
            cand --verbose 'report error message'
            cand -z 'separate output with NUL rather than newline'
            cand --zero 'separate output with NUL rather than newline'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
