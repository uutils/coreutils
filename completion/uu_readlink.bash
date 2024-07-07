_uu_readlink() {
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
                cmd="uu_readlink"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_readlink)
            opts="-f -e -m -n -q -s -v -z -h -V --canonicalize --canonicalize-existing --canonicalize-missing --no-newline --quiet --silent --verbose --zero --help --version [files]..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

complete -F _uu_readlink -o nosort -o bashdefault -o default uu_readlink
