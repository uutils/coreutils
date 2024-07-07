
use builtin;
use str;

set edit:completion:arg-completer[uu_rm] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_rm'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_rm'= {
            cand --interactive 'prompt according to WHEN: never, once (-I), or always (-i). Without WHEN, prompts always'
            cand -f 'ignore nonexistent files and arguments, never prompt'
            cand --force 'ignore nonexistent files and arguments, never prompt'
            cand -i 'prompt before every removal'
            cand -I 'prompt once before removing more than three files, or when removing recursively. Less intrusive than -i, while still giving some protection against most mistakes'
            cand --one-file-system 'when removing a hierarchy recursively, skip any directory that is on a file system different from that of the corresponding command line argument (NOT IMPLEMENTED)'
            cand --no-preserve-root 'do not treat ''/'' specially'
            cand --preserve-root 'do not remove ''/'' (default)'
            cand -r 'remove directories and their contents recursively'
            cand -R 'remove directories and their contents recursively'
            cand --recursive 'remove directories and their contents recursively'
            cand -d 'remove empty directories'
            cand --dir 'remove empty directories'
            cand -v 'explain what is being done'
            cand --verbose 'explain what is being done'
            cand --presume-input-tty 'presume-input-tty'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
