
use builtin;
use str;

set edit:completion:arg-completer[uu_cp] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_cp'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_cp'= {
            cand -t 'copy all SOURCE arguments into target-directory'
            cand --target-directory 'copy all SOURCE arguments into target-directory'
            cand --backup 'make a backup of each existing destination file'
            cand -S 'override the usual backup suffix'
            cand --suffix 'override the usual backup suffix'
            cand --update 'move only when the SOURCE file is newer than the destination file or when the destination file is missing'
            cand --reflink 'control clone/CoW copies. See below'
            cand --preserve 'Preserve the specified attributes (default: mode, ownership (unix only), timestamps), if possible additional attributes: context, links, xattr, all'
            cand --no-preserve 'don''t preserve the specified attributes'
            cand --sparse 'control creation of sparse files. See below'
            cand --context 'NotImplemented: set SELinux security context of destination file to default type'
            cand -T 'Treat DEST as a regular file and not a directory'
            cand --no-target-directory 'Treat DEST as a regular file and not a directory'
            cand -i 'ask before overwriting files'
            cand --interactive 'ask before overwriting files'
            cand -l 'hard-link files instead of copying'
            cand --link 'hard-link files instead of copying'
            cand -n 'don''t overwrite a file that already exists'
            cand --no-clobber 'don''t overwrite a file that already exists'
            cand -R 'copy directories recursively'
            cand -r 'copy directories recursively'
            cand --recursive 'copy directories recursively'
            cand --strip-trailing-slashes 'remove any trailing slashes from each SOURCE argument'
            cand --debug 'explain how a file is copied. Implies -v'
            cand -v 'explicitly state what is being done'
            cand --verbose 'explicitly state what is being done'
            cand -s 'make symbolic links instead of copying'
            cand --symbolic-link 'make symbolic links instead of copying'
            cand -f 'if an existing destination file cannot be opened, remove it and try again (this option is ignored when the -n option is also used). Currently not implemented for Windows.'
            cand --force 'if an existing destination file cannot be opened, remove it and try again (this option is ignored when the -n option is also used). Currently not implemented for Windows.'
            cand --remove-destination 'remove each existing destination file before attempting to open it (contrast with --force). On Windows, currently only works for writeable files.'
            cand -b 'like --backup but does not accept an argument'
            cand -u 'like --update but does not accept an argument'
            cand --attributes-only 'Don''t copy the file data, just the attributes'
            cand -p 'same as --preserve=mode,ownership(unix only),timestamps'
            cand --preserve-default-attributes 'same as --preserve=mode,ownership(unix only),timestamps'
            cand --parents 'use full source file name under DIRECTORY'
            cand -P 'never follow symbolic links in SOURCE'
            cand --no-dereference 'never follow symbolic links in SOURCE'
            cand -L 'always follow symbolic links in SOURCE'
            cand --dereference 'always follow symbolic links in SOURCE'
            cand -H 'follow command-line symbolic links in SOURCE'
            cand -a 'Same as -dR --preserve=all'
            cand --archive 'Same as -dR --preserve=all'
            cand -d 'same as --no-dereference --preserve=links'
            cand -x 'stay on this file system'
            cand --one-file-system 'stay on this file system'
            cand --copy-contents 'NotImplemented: copy contents of special files when recursive'
            cand -g 'Display a progress bar. 
Note: this feature is not supported by GNU coreutils.'
            cand --progress 'Display a progress bar. 
Note: this feature is not supported by GNU coreutils.'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
