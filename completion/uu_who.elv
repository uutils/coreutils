
use builtin;
use str;

set edit:completion:arg-completer[uu_who] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_who'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_who'= {
            cand -a 'same as -b -d --login -p -r -t -T -u'
            cand --all 'same as -b -d --login -p -r -t -T -u'
            cand -b 'time of last system boot'
            cand --boot 'time of last system boot'
            cand -d 'print dead processes'
            cand --dead 'print dead processes'
            cand -H 'print line of column headings'
            cand --heading 'print line of column headings'
            cand -l 'print system login processes'
            cand --login 'print system login processes'
            cand --lookup 'attempt to canonicalize hostnames via DNS'
            cand -m 'only hostname and user associated with stdin'
            cand -p 'print active processes spawned by init'
            cand --process 'print active processes spawned by init'
            cand -q 'all login names and number of users logged on'
            cand --count 'all login names and number of users logged on'
            cand -r 'print current runlevel'
            cand --runlevel 'print current runlevel'
            cand -s 'print only name, line, and time (default)'
            cand --short 'print only name, line, and time (default)'
            cand -t 'print last system clock change'
            cand --time 'print last system clock change'
            cand -u 'list users logged in'
            cand --users 'list users logged in'
            cand -T 'add user''s message status as +, - or ?'
            cand -w 'add user''s message status as +, - or ?'
            cand --mesg 'add user''s message status as +, - or ?'
            cand --message 'add user''s message status as +, - or ?'
            cand --writable 'add user''s message status as +, - or ?'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
