_uu_who() {
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
                cmd="uu_who"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_who)
            opts="-a -b -d -H -l -m -p -q -r -s -t -u -w -T -h -V --all --boot --dead --heading --login --lookup --process --count --runlevel --short --time --users --message --writable --mesg --help --version [FILE]..."
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

complete -F _uu_who -o nosort -o bashdefault -o default uu_who
