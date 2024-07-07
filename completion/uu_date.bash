_uu_date() {
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
                cmd="uu_date"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_date)
            opts="-d -f -I -R -r -s -u -h -V --date --file --iso-8601 --rfc-email --rfc-3339 --debug --reference --set --universal --help --version [format]"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --date)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -f)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --iso-8601)
                    COMPREPLY=($(compgen -W "date hours minutes seconds ns" -- "${cur}"))
                    return 0
                    ;;
                -I)
                    COMPREPLY=($(compgen -W "date hours minutes seconds ns" -- "${cur}"))
                    return 0
                    ;;
                --rfc-3339)
                    COMPREPLY=($(compgen -W "date seconds ns" -- "${cur}"))
                    return 0
                    ;;
                --reference)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -r)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --set)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -s)
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

complete -F _uu_date -o nosort -o bashdefault -o default uu_date
