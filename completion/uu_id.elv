
use builtin;
use str;

set edit:completion:arg-completer[uu_id] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_id'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_id'= {
            cand -A 'Display the process audit user ID and other process audit properties,
which requires privilege (not available on Linux).'
            cand -u 'Display only the effective user ID as a number.'
            cand --user 'Display only the effective user ID as a number.'
            cand -g 'Display only the effective group ID as a number'
            cand --group 'Display only the effective group ID as a number'
            cand -G 'Display only the different group IDs as white-space separated numbers, in no particular order.'
            cand --groups 'Display only the different group IDs as white-space separated numbers, in no particular order.'
            cand -p 'Make the output human-readable. Each display is on a separate line.'
            cand -n 'Display the name of the user or group ID for the -G, -g and -u options instead of the number.
If any of the ID numbers cannot be mapped into names, the number will be displayed as usual.'
            cand --name 'Display the name of the user or group ID for the -G, -g and -u options instead of the number.
If any of the ID numbers cannot be mapped into names, the number will be displayed as usual.'
            cand -P 'Display the id as a password file entry.'
            cand -r 'Display the real ID for the -G, -g and -u options instead of the effective ID.'
            cand --real 'Display the real ID for the -G, -g and -u options instead of the effective ID.'
            cand -z 'delimit entries with NUL characters, not whitespace;
not permitted in default format'
            cand --zero 'delimit entries with NUL characters, not whitespace;
not permitted in default format'
            cand -Z 'print only the security context of the process (not enabled)'
            cand --context 'print only the security context of the process (not enabled)'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
