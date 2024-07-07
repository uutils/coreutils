_uu_du() {
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
                cmd="uu_du"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_du)
            opts="-a -B -b -c -d -h -k -l -L -H -D -P -m -0 -S -s -x -t -v -X -V --help --all --apparent-size --block-size --bytes --total --max-depth --human-readable --inodes --count-links --dereference --dereference-args --no-dereference --null --separate-dirs --summarize --si --one-file-system --threshold --verbose --exclude --exclude-from --files0-from --time --time-style --version [FILE]..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --block-size)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -B)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-depth)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --threshold)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -t)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --exclude)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --exclude-from)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -X)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --files0-from)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --time)
                    COMPREPLY=($(compgen -W "atime ctime creation" -- "${cur}"))
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

complete -F _uu_du -o nosort -o bashdefault -o default uu_du
