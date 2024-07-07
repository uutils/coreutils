
use builtin;
use str;

set edit:completion:arg-completer[uu_realpath] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_realpath'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_realpath'= {
            cand --relative-to 'print the resolved path relative to DIR'
            cand --relative-base 'print absolute paths unless paths below DIR'
            cand -q 'Do not print warnings for invalid paths'
            cand --quiet 'Do not print warnings for invalid paths'
            cand -s 'Only strip ''.'' and ''..'' components, but don''t resolve symbolic links'
            cand --strip 'Only strip ''.'' and ''..'' components, but don''t resolve symbolic links'
            cand --no-symlinks 'Only strip ''.'' and ''..'' components, but don''t resolve symbolic links'
            cand -z 'Separate output filenames with \0 rather than newline'
            cand --zero 'Separate output filenames with \0 rather than newline'
            cand -L 'resolve ''..'' components before symlinks'
            cand --logical 'resolve ''..'' components before symlinks'
            cand -P 'resolve symlinks as encountered (default)'
            cand --physical 'resolve symlinks as encountered (default)'
            cand -e 'canonicalize by following every symlink in every component of the given name recursively, all components must exist'
            cand --canonicalize-existing 'canonicalize by following every symlink in every component of the given name recursively, all components must exist'
            cand -m 'canonicalize by following every symlink in every component of the given name recursively, without requirements on components existence'
            cand --canonicalize-missing 'canonicalize by following every symlink in every component of the given name recursively, without requirements on components existence'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
