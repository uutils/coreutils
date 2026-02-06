#!/bin/bash

# spell-checker:ignore nextest watchplus PIPESTATUS

echo "PATH: $PATH"

export PATH=$HOME/.cargo/bin:$PATH
export RUST_BACKTRACE=full
export CARGO_TERM_COLOR=always
export CARGO_INCREMENTAL=0

echo "PATH: $PATH"

run_with_retry() {
    tries=$1
    shift 1

    for i in $(seq 1 $tries); do
        echo "Try #$i of $tries: run $*"
        "$@" && echo "Done in try#$i" && return 0
    done

    exit_code=$?

    echo "Still failing after $tries. Code: $exit_code"

    return $exit_code
}

run_tests_in_subprocess() (

    # limit virtual memory to 3GB to avoid that OS kills sshd
    ulimit -v $((1024 * 1024 * 3))

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

    nextest_params=(--profile ci --hide-progress-bar --features feat_os_unix_android)

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

# run sub-shell to be able to use ulimit without affecting the sshd
run_tests_in_subprocess
