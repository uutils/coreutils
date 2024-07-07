
use builtin;
use str;

set edit:completion:arg-completer[uu_chroot] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_chroot'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_chroot'= {
            cand -u 'User (ID or name) to switch before running the program'
            cand --user 'User (ID or name) to switch before running the program'
            cand -g 'Group (ID or name) to switch to'
            cand --group 'Group (ID or name) to switch to'
            cand -G 'Comma-separated list of groups to switch to'
            cand --groups 'Comma-separated list of groups to switch to'
            cand --userspec 'Colon-separated user and group to switch to. Same as -u USER -g GROUP. Userspec has higher preference than -u and/or -g'
            cand --skip-chdir 'Use this option to not change the working directory to / after changing the root directory to newroot, i.e., inside the chroot.'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
