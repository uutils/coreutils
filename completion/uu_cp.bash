_uu_cp() {
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
                cmd="uu_cp"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_cp)
            opts="-t -T -i -l -n -r -R -v -s -f -b -S -u -p -P -L -H -a -d -x -g -h -V --target-directory --no-target-directory --interactive --link --no-clobber --recursive --strip-trailing-slashes --debug --verbose --symbolic-link --force --remove-destination --backup --suffix --update --reflink --attributes-only --preserve --preserve-default-attributes --no-preserve --parents --no-dereference --dereference --archive --one-file-system --sparse --copy-contents --context --progress --help --version [paths]..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --target-directory)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -t)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
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
                --reflink)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --preserve)
                    COMPREPLY=($(compgen -W "mode ownership timestamps context links xattr all" -- "${cur}"))
                    return 0
                    ;;
                --no-preserve)
                    COMPREPLY=($(compgen -W "mode ownership timestamps context links xattr all" -- "${cur}"))
                    return 0
                    ;;
                --sparse)
                    COMPREPLY=($(compgen -W "never auto always" -- "${cur}"))
                    return 0
                    ;;
                --context)
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

complete -F _uu_cp -o nosort -o bashdefault -o default uu_cp
