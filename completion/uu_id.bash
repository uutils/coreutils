_uu_id() {
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
                cmd="uu_id"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_id)
            opts="-A -u -g -G -p -n -P -r -z -Z -h -V --user --group --groups --name --real --zero --context --help --version [USER]..."
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

complete -F _uu_id -o nosort -o bashdefault -o default uu_id
