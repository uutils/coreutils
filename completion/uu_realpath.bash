_uu_realpath() {
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
                cmd="uu_realpath"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_realpath)
            opts="-q -s -z -L -P -e -m -h -V --quiet --no-symlinks --strip --zero --logical --physical --canonicalize-existing --canonicalize-missing --relative-to --relative-base --help --version <files>..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --relative-to)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --relative-base)
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

complete -F _uu_realpath -o nosort -o bashdefault -o default uu_realpath
