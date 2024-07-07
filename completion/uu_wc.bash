_uu_wc() {
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
                cmd="uu_wc"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_wc)
            opts="-c -m -l -L -w -h -V --bytes --chars --files0-from --lines --max-line-length --total --words --help --version [files]..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --files0-from)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --total)
                    COMPREPLY=($(compgen -W "auto always only never" -- "${cur}"))
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

complete -F _uu_wc -o nosort -o bashdefault -o default uu_wc
