_uu_chroot() {
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
                cmd="uu_chroot"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_chroot)
            opts="-u -g -G -h -V --user --group --groups --userspec --skip-chdir --help --version <newroot> [command]..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --user)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -u)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --group)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -g)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --groups)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -G)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --userspec)
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

complete -F _uu_chroot -o nosort -o bashdefault -o default uu_chroot
