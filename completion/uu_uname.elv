
use builtin;
use str;

set edit:completion:arg-completer[uu_uname] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_uname'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_uname'= {
            cand -a 'Behave as though all of the options -mnrsvo were specified.'
            cand --all 'Behave as though all of the options -mnrsvo were specified.'
            cand -s 'print the kernel name.'
            cand --kernel-name 'print the kernel name.'
            cand -n 'print the nodename (the nodename may be a name that the system is known by to a communications network).'
            cand --nodename 'print the nodename (the nodename may be a name that the system is known by to a communications network).'
            cand -r 'print the operating system release.'
            cand --kernel-release 'print the operating system release.'
            cand -v 'print the operating system version.'
            cand --kernel-version 'print the operating system version.'
            cand -m 'print the machine hardware name.'
            cand --machine 'print the machine hardware name.'
            cand -o 'print the operating system name.'
            cand --operating-system 'print the operating system name.'
            cand -p 'print the processor type (non-portable)'
            cand --processor 'print the processor type (non-portable)'
            cand -i 'print the hardware platform (non-portable)'
            cand --hardware-platform 'print the hardware platform (non-portable)'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
