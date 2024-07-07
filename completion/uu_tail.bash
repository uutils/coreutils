_uu_tail() {
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
                cmd="uu_tail"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_tail)
            opts="-c -f -n -q -s -v -z -F -h -V --bytes --follow --lines --pid --silent --quiet --sleep-interval --max-unchanged-stats --verbose --zero-terminated --use-polling --retry --presume-input-pipe --help --version [files]..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --bytes)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --follow)
                    COMPREPLY=($(compgen -W "descriptor name" -- "${cur}"))
                    return 0
                    ;;
                -f)
                    COMPREPLY=($(compgen -W "descriptor name" -- "${cur}"))
                    return 0
                    ;;
                --lines)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -n)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --pid)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sleep-interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -s)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-unchanged-stats)
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

complete -F _uu_tail -o nosort -o bashdefault -o default uu_tail
