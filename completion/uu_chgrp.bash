_uu_chgrp() {
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
                cmd="uu_chgrp"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_chgrp)
            opts="-c -f -v -h -R -H -P -L -V --help --changes --silent --quiet --verbose --dereference --no-dereference --preserve-root --no-preserve-root --reference --recursive --version"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --reference)
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

complete -F _uu_chgrp -o nosort -o bashdefault -o default uu_chgrp
