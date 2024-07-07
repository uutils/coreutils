_uu_cksum() {
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
                cmd="uu_cksum"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        uu_cksum)
            opts="-a -l -c -t -b -w -h -V --algorithm --untagged --tag --length --raw --strict --check --base64 --text --binary --warn --status --quiet --ignore-missing --help --version [file]..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --algorithm)
                    COMPREPLY=($(compgen -W "sysv bsd crc md5 sha1 sha3 sha224 sha256 sha384 sha512 blake2b blake3 sm3 shake128 shake256" -- "${cur}"))
                    return 0
                    ;;
                -a)
                    COMPREPLY=($(compgen -W "sysv bsd crc md5 sha1 sha3 sha224 sha256 sha384 sha512 blake2b blake3 sm3 shake128 shake256" -- "${cur}"))
                    return 0
                    ;;
                --length)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -l)
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

complete -F _uu_cksum -o nosort -o bashdefault -o default uu_cksum
