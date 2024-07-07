_uu_ls() {
    local i cur prev opts cmd
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    cmd=""
    opts=""

    for i in ${COMP_WORDS[@]}
    do
        case "${cmd},${i}" in
            ",$1")
                cmd="uu_ls"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_ls)
            opts="-C -l -x -T -m -D -1 -o -g -n -N -b -Q -q -c -u -I -B -S -t -v -X -U -L -H -G -a -A -d -h -k -i -r -R -w -s -F -p -Z -V --help --format --long --tabsize --zero --dired --hyperlink --numeric-uid-gid --quoting-style --literal --escape --quote-name --hide-control-chars --show-control-chars --time --hide --ignore --ignore-backups --sort --dereference --dereference-command-line-symlink-to-dir --dereference-command-line --no-group --author --all --almost-all --directory --human-readable --kibibytes --si --block-size --inode --reverse --recursive --width --size --color --indicator-style --classify --file-type --time-style --full-time --context --group-directories-first --version [paths]..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --format)
                    COMPREPLY=($(compgen -W "long verbose single-column columns vertical across horizontal commas" -- "${cur}"))
                    return 0
                    ;;
                --tabsize)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -T)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --hyperlink)
                    COMPREPLY=($(compgen -W "always auto never" -- "${cur}"))
                    return 0
                    ;;
                --quoting-style)
                    COMPREPLY=($(compgen -W "literal shell shell-escape shell-always shell-escape-always c escape" -- "${cur}"))
                    return 0
                    ;;
                --time)
                    COMPREPLY=($(compgen -W "atime ctime birth" -- "${cur}"))
                    return 0
                    ;;
                --hide)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ignore)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -I)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sort)
                    COMPREPLY=($(compgen -W "name none time size version extension width" -- "${cur}"))
                    return 0
                    ;;
                --block-size)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -w)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "always auto never" -- "${cur}"))
                    return 0
                    ;;
                --indicator-style)
                    COMPREPLY=($(compgen -W "none slash file-type classify" -- "${cur}"))
                    return 0
                    ;;
                --classify)
                    COMPREPLY=($(compgen -W "always auto never" -- "${cur}"))
                    return 0
                    ;;
                -F)
                    COMPREPLY=($(compgen -W "always auto never" -- "${cur}"))
                    return 0
                    ;;
                --time-style)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

complete -F _uu_ls -o nosort -o bashdefault -o default uu_ls
