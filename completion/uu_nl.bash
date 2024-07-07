_uu_nl() {
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
                cmd="uu_nl"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_nl)
            opts="-b -d -f -h -i -l -n -p -s -v -w -V --help --body-numbering --section-delimiter --footer-numbering --header-numbering --line-increment --join-blank-lines --number-format --no-renumber --number-separator --starting-line-number --number-width --version [file]..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --body-numbering)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -b)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --section-delimiter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --footer-numbering)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -f)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --header-numbering)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -h)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --line-increment)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --join-blank-lines)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -l)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --number-format)
                    COMPREPLY=($(compgen -W "ln rn rz" -- "${cur}"))
                    return 0
                    ;;
                -n)
                    COMPREPLY=($(compgen -W "ln rn rz" -- "${cur}"))
                    return 0
                    ;;
                --number-separator)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -s)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --starting-line-number)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -v)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --number-width)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -w)
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

complete -F _uu_nl -o nosort -o bashdefault -o default uu_nl
