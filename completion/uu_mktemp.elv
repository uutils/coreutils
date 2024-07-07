
use builtin;
use str;

set edit:completion:arg-completer[uu_mktemp] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_mktemp'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_mktemp'= {
            cand --suffix 'append SUFFIX to TEMPLATE; SUFFIX must not contain a path separator. This option is implied if TEMPLATE does not end with X.'
            cand -p 'short form of --tmpdir'
            cand --tmpdir 'interpret TEMPLATE relative to DIR; if DIR is not specified, use $TMPDIR ($TMP on windows) if set, else /tmp. With this option, TEMPLATE must not be an absolute name; unlike with -t, TEMPLATE may contain slashes, but mktemp creates only the final component'
            cand -d 'Make a directory instead of a file'
            cand --directory 'Make a directory instead of a file'
            cand -u 'do not create anything; merely print a name (unsafe)'
            cand --dry-run 'do not create anything; merely print a name (unsafe)'
            cand -q 'Fail silently if an error occurs.'
            cand --quiet 'Fail silently if an error occurs.'
            cand -t 'Generate a template (using the supplied prefix and TMPDIR (TMP on windows) if set) to create a filename template [deprecated]'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
