
use builtin;
use str;

set edit:completion:arg-completer[uu_install] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_install'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_install'= {
            cand --backup 'make a backup of each existing destination file'
            cand -g 'set group ownership, instead of process''s current group'
            cand --group 'set group ownership, instead of process''s current group'
            cand -m 'set permission mode (as in chmod), instead of rwxr-xr-x'
            cand --mode 'set permission mode (as in chmod), instead of rwxr-xr-x'
            cand -o 'set ownership (super-user only)'
            cand --owner 'set ownership (super-user only)'
            cand --strip-program 'program used to strip binaries (no action Windows)'
            cand -S 'override the usual backup suffix'
            cand --suffix 'override the usual backup suffix'
            cand -t 'move all SOURCE arguments into DIRECTORY'
            cand --target-directory 'move all SOURCE arguments into DIRECTORY'
            cand -b 'like --backup but does not accept an argument'
            cand -c 'ignored'
            cand -C 'compare each pair of source and destination files, and in some cases, do not modify the destination at all'
            cand --compare 'compare each pair of source and destination files, and in some cases, do not modify the destination at all'
            cand -d 'treat all arguments as directory names. create all components of the specified directories'
            cand --directory 'treat all arguments as directory names. create all components of the specified directories'
            cand -D 'create all leading components of DEST except the last, then copy SOURCE to DEST'
            cand -p 'apply access/modification times of SOURCE files to corresponding destination files'
            cand --preserve-timestamps 'apply access/modification times of SOURCE files to corresponding destination files'
            cand -s 'strip symbol tables (no action Windows)'
            cand --strip 'strip symbol tables (no action Windows)'
            cand -T '(unimplemented) treat DEST as a normal file'
            cand --no-target-directory '(unimplemented) treat DEST as a normal file'
            cand -v 'explain what is being done'
            cand --verbose 'explain what is being done'
            cand -P '(unimplemented) preserve security context'
            cand --preserve-context '(unimplemented) preserve security context'
            cand -Z '(unimplemented) set security context of files and directories'
            cand --context '(unimplemented) set security context of files and directories'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
