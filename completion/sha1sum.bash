_sha1sum() {
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
                cmd="sha1sum"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        sha1sum)
            opts="-b -c -t -q -s -w -z -h -V --binary --check --tag --text --quiet --status --strict --ignore-missing --warn --zero --help --version [file]..."
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

complete -F _sha1sum -o nosort -o bashdefault -o default sha1sum
