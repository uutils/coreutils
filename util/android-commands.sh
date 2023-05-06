#!/bin/bash
# spell-checker:ignore termux keyevent sdcard binutils unmatch adb's dumpsys logcat pkill nextest logfile

# There are three shells: the host's, adb, and termux. Only adb lets us run
# commands directly on the emulated device, only termux provides a GNU
# environment on the emulated device (to e.g. run cargo). So we use adb to
# launch termux, then to send keystrokes to it while it's running.
# This means that the commands sent to termux are first parsed as arguments in
# this shell, then as arguments in the adb shell, before finally being used as
# text inputs to the app. Hence, the "'wrapping'" on those commands.
# There's no way to get any direct feedback from termux, so every time we run a
# command on it, we make sure it creates a unique *.probe file which is polled
# every 30 seconds together with the current output of the command in a *.log file.
# The contents of the probe file are used as a return code: 0 on success, some
# other number for errors (an empty file is basically the same as 0). Note that
# the return codes are text, not raw bytes.

this_repo="$(dirname "$(dirname -- "$(readlink -- "${0}")")")"
cache_dir_name="__rust_cache__"

help() {
    echo \
        "Usage: $0 COMMAND [ARG]

where COMMAND is one of:
  init          download termux and initialize the emulator image
  snapshot APK  install APK and dependencies on an emulator to prep a snapshot
                (you can, but probably don't want to, run this for physical
                devices -- just set up termux and the dependencies yourself)
  sync_host [REPO]
                push the repo at REPO to the device, deleting and restoring all symlinks (locally)
                in the process; The cached rust directories are restored, too; by default, REPO is:
                $this_repo
  sync_image [REPO]
                copy the repo/target and the HOME/.cargo directories from the device back to the
                host; by default, REPO is: $this_repo
  build         run \`cargo build --features feat_os_unix_android\` on the
                device
  tests         run \`cargo test --features feat_os_unix_android\` on the
                device

If you have multiple devices, use the ANDROID_SERIAL environment variable to
specify which to connect to."
}

hit_enter() {
    adb shell input keyevent 66
}

exit_termux() {
    adb shell input text "exit" && hit_enter && hit_enter
}

launch_termux() {
    echo "launching termux"
    if ! adb shell 'am start -n com.termux/.HomeActivity'; then
        echo "failed to launch termux"
        exit 1
    fi
    # the emulator can sometimes be a little slow to launch the app
    while ! adb shell 'ls /sdcard/launch.probe' 2>/dev/null; do
        echo "waiting for launch.probe"
        sleep 5
        adb shell input text 'touch\ /sdcard/launch.probe' && hit_enter
    done
    echo "found launch.probe"
    adb shell 'rm /sdcard/launch.probe' && echo "removed launch.probe"
}

# Usage: run_termux_command
#
# Runs the command specified in $1 in a termux shell, polling for the probe specified in $2 (and the
# current output). If polling the probe succeeded the command is considered to have finished. This
# method prints the current stdout and stderr of the command every SLEEP_INTERVAL seconds and
# finishes a command run with a summary. It returns with the exit code of the probe if specified as
# file content of the probe.
#
# Positional arguments
# $1                The command to execute in the termux shell
# $2                The path to the probe. The file name must end with `.probe`
#
# It's possible to overwrite settings by specifying the setting the variable before calling this
# method (Default in parentheses):
# keep_log  0|1     Keeps the logs after running the command if set to 1. The log file name is
#                   derived from the probe file name (the last component of the path) and
#                   `.probe` replaced with `.log. (0)
# debug     0|1     Adds additional debugging output to the log file if set to 1. (1)
# timeout   SECONDS The timeout in full SECONDS for the command to complete before giving up. (3600)
# retries   RETRIES The number of retries for trying to fix possible issues when we're not receiving
#                   any progress from the emulator. (3)
# sleep_interval
#           SECONDS The time interval in full SECONDS between polls for the probe and the current
#           output. (5)
run_termux_command() {
    # shellcheck disable=SC2155
    local command="$(echo "$1" | sed -E "s/^['](.*)[']$/\1/")" # text of the escaped command, including creating the probe!
    local probe="$2"                                           # unique file that indicates the command is complete
    local keep_log=${keep_log:-0}
    local debug=${debug:-1}

    log_name="$(basename -s .probe "${probe}").log" # probe name must have suffix .probe
    log_file="/sdcard/${log_name}"
    log_read="${log_name}.read"
    echo 0 >"${log_read}"
    if [[ $debug -eq 1 ]]; then
        shell_command="'set -x; { ${command}; } &> ${log_file}; set +x'"
    else
        shell_command="'{ ${command}; } &> ${log_file}'"
    fi

    launch_termux
    echo "Running command: ${command}"
    start=$(date +%s)
    adb shell input text "$shell_command" && sleep 3 && hit_enter
    # just for safety wait a little bit before polling for the probe and the log file
    sleep 5

    local timeout=${timeout:-3600}
    local retries=${retries:-10}
    local sleep_interval=${sleep_interval:-10}
    try_fix=3
    echo "run_termux_command with timeout=$timeout / retries=$retries / sleep_interval=$sleep_interval"
    while ! adb shell "ls $probe" 2>/dev/null; do
        echo -n "Waiting for $probe: "

        if [[ -e "$log_name" ]]; then
            rm "$log_name"
        fi

        adb pull "$log_file" . || try_fix=$((try_fix - 1))
        if [[ -e "$log_name" ]]; then
            tail -n +"$(<"$log_read")" "$log_name"
            echo
            wc -l <"${log_name}" | tr -d "[:space:]" >"$log_read"
        fi

        if [[ retries -le 0 ]]; then
            echo "Maximum retries reached running command. Aborting ..."
            return 1
        elif [[ try_fix -le 0 ]]; then
            retries=$((retries - 1))
            try_fix=3
            # Since there is no output, there is no way to know what is happening inside. See if
            # hitting the enter key solves the issue, sometimes the github runner is just a little
            # bit slow.
            echo "No output received. Trying to fix the issue ... (${retries} retries left)"
            hit_enter
        fi

        sleep "$sleep_interval"
        timeout=$((timeout - sleep_interval))

        if [[ $timeout -le 0 ]]; then
            echo "Timeout reached running command. Aborting ..."
            return 1
        fi
    done
    end=$(date +%s)

    return_code=$(adb shell "cat $probe") || return_code=0
    adb shell "rm ${probe}"

    adb pull "$log_file" .
    echo "==================================== SUMMARY ==================================="
    echo "Command: ${command}"
    echo "Finished in $((end - start)) seconds."
    echo "Output was:"
    cat "$log_name"
    echo "Return code: $return_code"
    echo "================================================================================"

    adb shell "rm ${log_file}"
    [[ $keep_log -ne 1 ]] && rm -f "$log_name"
    rm -f "$log_read" "$probe"

    # shellcheck disable=SC2086
    return $return_code
}

init() {
    arch="$1"
    api_level="$2"
    termux="$3"

    # shellcheck disable=SC2015
    wget "https://github.com/termux/termux-app/releases/download/${termux}/termux-app_${termux}+github-debug_${arch}.apk" &&
        snapshot "termux-app_${termux}+github-debug_${arch}.apk" &&
        hash_rustc &&
        exit_termux &&
        adb -s emulator-5554 emu avd snapshot save "${api_level}-${arch}+termux-${termux}" &&
        echo "Emulator image created." || {
        pkill -9 qemu-system-x86_64
        return 1
    }
    pkill -9 qemu-system-x86_64 || true
}

snapshot() {
    apk="$1"
    echo "Running snapshot"
    adb install -g "$apk"

    echo "Prepare and install system packages"
    probe='/sdcard/pkg.probe'
    command="'mkdir -vp ~/.cargo/bin; yes | pkg install rust binutils openssl tar -y; echo \$? > $probe'"
    run_termux_command "$command" "$probe" || return

    echo "Installing cargo-nextest"
    probe='/sdcard/nextest.probe'
    # We need to install nextest via cargo currently, since there is no pre-built binary for android x86
    command="'\
export CARGO_TERM_COLOR=always; \
cargo install cargo-nextest; \
echo \$? > $probe'"
    run_termux_command "$command" "$probe"
    return_code=$?

    echo "Info about cargo and rust"
    probe='/sdcard/info.probe'
    command="'echo \$HOME; \
PATH=\$HOME/.cargo/bin:\$PATH; \
export PATH; \
echo \$PATH; \
pwd; \
command -v rustc && rustc -Vv; \
ls -la ~/.cargo/bin; \
cargo --list; \
cargo nextest --version; \
touch $probe'"
    run_termux_command "$command" "$probe"

    echo "Snapshot complete"
    # shellcheck disable=SC2086
    return $return_code
}

sync_host() {
    repo="$1"
    cache_home="${HOME}/${cache_dir_name}"
    cache_dest="/sdcard/${cache_dir_name}"

    echo "Running sync host -> image: ${repo}"

    # android doesn't allow symlinks on shared dirs, and adb can't selectively push files
    symlinks=$(find "$repo" -type l)
    # dash doesn't support process substitution :(
    echo "$symlinks" | sort >symlinks

    git -C "$repo" diff --name-status | cut -f 2 >modified
    modified_links=$(join symlinks modified)
    if [ -n "$modified_links" ]; then
        echo "You have modified symlinks. Either stash or commit them, then try again: $modified_links"
        exit 1
    fi
    #shellcheck disable=SC2086
    if ! git ls-files --error-unmatch $symlinks >/dev/null; then
        echo "You have untracked symlinks. Either remove or commit them, then try again."
        exit 1
    fi

    #shellcheck disable=SC2086
    rm $symlinks
    # adb's shell user only has access to shared dirs...
    adb push -a "$repo" /sdcard/coreutils
    [[ -e "$cache_home" ]] && adb push -a "$cache_home" "$cache_dest"

    #shellcheck disable=SC2086
    git -C "$repo" checkout $symlinks

    # ...but shared dirs can't build, so move it home as termux
    probe='/sdcard/sync.probe'
    command="'mv /sdcard/coreutils ~/; \
cd ~/coreutils; \
if [[ -e ${cache_dest} ]]; then \
rm -rf ~/.cargo ./target; \
tar xzf ${cache_dest}/cargo.tgz -C ~/; \
ls -la ~/.cargo; \
tar xzf ${cache_dest}/target.tgz; \
ls -la ./target; \
rm -rf ${cache_dest}; \
fi; \
touch $probe'"
    run_termux_command "$command" "$probe" || return

    echo "Finished sync host -> image: ${repo}"
}

sync_image() {
    repo="$1"
    cache_home="${HOME}/${cache_dir_name}"
    cache_dest="/sdcard/${cache_dir_name}"

    echo "Running sync image -> host: ${repo}"

    probe='/sdcard/cache.probe'
    command="'rm -rf /sdcard/coreutils ${cache_dest}; \
mkdir -p ${cache_dest}; \
cd ${cache_dest}; \
tar czf cargo.tgz -C ~/ .cargo; \
tar czf target.tgz -C ~/coreutils target; \
ls -la ${cache_dest}; \
echo \$? > ${probe}'"
    run_termux_command "$command" "$probe" || return

    rm -rf "$cache_home"
    adb pull -a "$cache_dest" "$cache_home" || return

    echo "Finished sync image -> host: ${repo}"
}

build() {
    echo "Running build"

    probe='/sdcard/build.probe'
    command="'export CARGO_TERM_COLOR=always; \
export CARGO_INCREMENTAL=0; \
cd ~/coreutils && cargo build --features feat_os_unix_android; \
echo \$? >$probe'"
    run_termux_command "$command" "$probe" || return

    echo "Finished build"
}

tests() {
    echo "Running tests"

    probe='/sdcard/tests.probe'
    command="'export PATH=\$HOME/.cargo/bin:\$PATH; \
export RUST_BACKTRACE=1; \
export CARGO_TERM_COLOR=always; \
export CARGO_INCREMENTAL=0; \
cd ~/coreutils; \
timeout --preserve-status --verbose -k 1m 60m \
cargo nextest run --profile ci --hide-progress-bar --features feat_os_unix_android; \
echo \$? >$probe'"
    run_termux_command "$command" "$probe" || return

    echo "Finished tests"
}

hash_rustc() {
    probe='/sdcard/rustc.probe'
    tmp_hash="__rustc_hash__.tmp"
    hash="__rustc_hash__"

    echo "Hashing rustc version: ${HOME}/${hash}"

    command="'rustc -Vv; echo \$? > ${probe}'"
    keep_log=1
    debug=0
    run_termux_command "$command" "$probe" || return
    rm -f "$tmp_hash"
    mv "rustc.log" "$tmp_hash" || return
    # sha256sum is not available. shasum is the macos native program.
    shasum -a 256 "$tmp_hash" | cut -f 1 -d ' ' | tr -d '[:space:]' >"${HOME}/${hash}" || return

    rm -f "$tmp_hash"

    echo "Finished hashing rustc version: ${HOME}/${hash}"
}

#adb logcat &
exit_code=0

if [ $# -eq 1 ]; then
    case "$1" in
        sync_host)
            sync_host "$this_repo"
            exit_code=$?
            ;;
        sync_image)
            sync_image "$this_repo"
            exit_code=$?
            ;;
        build)
            build
            exit_code=$?
            ;;
        tests)
            tests
            exit_code=$?
            ;;
        *) help ;;
    esac
elif [ $# -eq 2 ]; then
    case "$1" in
        snapshot)
            snapshot "$2"
            exit_code=$?
            ;;
        sync_host)
            sync_host "$2"
            exit_code=$?
            ;;
        sync_image)
            sync_image "$2"
            exit_code=$?
            ;;
        *)
            help
            exit 1
            ;;
    esac
elif [ $# -eq 4 ]; then
    case "$1" in
        init)
            shift
            init "$@"
            exit_code=$?
            ;;
        *)
            help
            exit 1
            ;;
    esac
else
    help
    exit_code=1
fi

#pkill adb
exit $exit_code
