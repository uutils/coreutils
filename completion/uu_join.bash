_uu_join() {
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
                cmd="uu_join"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_join)
            opts="-a -v -e -i -j -o -t -1 -2 -z -h -V --ignore-case --check-order --nocheck-order --header --zero-terminated --help --version <FILE1> <FILE2>"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                -a)
                    COMPREPLY=($(compgen -W "1 2" -- "${cur}"))
                    return 0
                    ;;
                -v)
                    COMPREPLY=($(compgen -W "1 2" -- "${cur}"))
                    return 0
                    ;;
                -e)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -j)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -t)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -1)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -2)
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

complete -F _uu_join -o nosort -o bashdefault -o default uu_join
