_uu_uniq() {
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
                cmd="uu_uniq"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_uniq)
            opts="-D -w -c -i -d -s -f -u -z -h -V --all-repeated --group --check-chars --count --ignore-case --repeated --skip-chars --skip-fields --unique --zero-terminated --help --version [files]..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --all-repeated)
                    COMPREPLY=($(compgen -W "none prepend separate" -- "${cur}"))
                    return 0
                    ;;
                -D)
                    COMPREPLY=($(compgen -W "none prepend separate" -- "${cur}"))
                    return 0
                    ;;
                --group)
                    COMPREPLY=($(compgen -W "separate prepend append both" -- "${cur}"))
                    return 0
                    ;;
                --check-chars)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -w)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --skip-chars)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -s)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --skip-fields)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -f)
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

complete -F _uu_uniq -o nosort -o bashdefault -o default uu_uniq
