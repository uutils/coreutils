_uu_mv() {
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
                cmd="uu_mv"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_mv)
            opts="-f -i -n -b -S -u -t -T -v -g -h -V --force --interactive --no-clobber --strip-trailing-slashes --backup --suffix --update --target-directory --no-target-directory --verbose --progress --help --version <files>..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --backup)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --suffix)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -S)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --update)
                    COMPREPLY=($(compgen -W "none all older" -- "${cur}"))
                    return 0
                    ;;
                --target-directory)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -t)
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

complete -F _uu_mv -o nosort -o bashdefault -o default uu_mv
