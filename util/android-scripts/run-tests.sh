#!/bin/bash

# spell-checker:ignore nextest watchplus PIPESTATUS

echo "PATH: $PATH"

export PATH=$HOME/.cargo/bin:$PATH
export RUST_BACKTRACE=full
export CARGO_TERM_COLOR=always
export CARGO_INCREMENTAL=0
export CARGO_BUILD_JOBS=2

echo "PATH: $PATH"

case "${1:-all}" in
    all) test_filter=() ;;
    cksum-wc) test_filter=(-E 'test(test_cksum::) | test(test_wc::)') ;;
    *) echo "Unknown test suite: $1" >&2; exit 1 ;;
esac

run_with_retry() {
    local tries=$1
    shift 1
    local exit_code=0

    for ((i = 1; i <= tries; i++)); do
        echo "Try #$i of $tries: run $*"
        if "$@"; then
            echo "Done in try#$i"
            return 0
        else
            exit_code=$?
        fi
    done

    echo "Still failing after $tries. Code: $exit_code"

    return $exit_code
}

run_tests_in_subprocess() (

    watchplus() {
        # call: watchplus <interval> <command>
        while true; do
            "${@:2}"
            sleep "$1"
        done
    }

    kill_all_background_jobs() {
        jobs -p | xargs -I{} kill -- {}
    }

    # observe (log) every 2 seconds the system resource usage to judge if we are at a limit
    watchplus 2 df -h &
    watchplus 2 free -hm &

    # Recent Android allocators reserve more virtual memory than a 3 GiB limit
    # allows. Limit parallelism to reduce memory pressure without capping addresses.
    nextest_params=(--profile ci --hide-progress-bar --test-threads 2 --features feat_os_unix_android "${test_filter[@]}")

    # run tests
    cd ~/coreutils && \
        run_with_retry 3 timeout --preserve-status --verbose -k 1m 10m \
            cargo nextest run --no-run "${nextest_params[@]}" &&
        timeout --preserve-status --verbose -k 1m 60m \
            cargo nextest run "${nextest_params[@]}"

    result=$?

    kill_all_background_jobs

    return $result
)

# Keep the resource monitors and their cleanup in a separate shell.
run_tests_in_subprocess
