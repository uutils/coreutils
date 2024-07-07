_uu_uname() {
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
                cmd="uu_uname"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_uname)
            opts="-a -s -n -r -v -m -o -p -i -h -V --all --kernel-name --nodename --kernel-release --kernel-version --machine --operating-system --processor --hardware-platform --help --version"
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

complete -F _uu_uname -o nosort -o bashdefault -o default uu_uname
