#!/usr/bin/env bash
# spell-checker:ignore termux keyevent sdcard binutils unmatch adb's dumpsys logcat pkill nextest logfile
# spell-checker:ignore screencap reinit PIPESTATUS keygen sourceslist

# There are four shells: the host's, adb, termux and termux via ssh.
# But only termux and termux via ssh provides a GNU environment on the
# emulated device (to e.g. run cargo).
# Initially, only adb lets us run commands directly on the emulated device.
# Thus we first establish a ssh connection which then can be used to access
# the termux shell directly, getting output and return code as usual.
# So we use adb to launch termux, then to send keystrokes to it while it's running.
# This way we install sshd and a public key from the host. After that we can
# use ssh to directly run commands in termux environment.

# Before ssh, we need to consider some inconvenient, limiting specialties:
# The commands sent to termux via adb keystrokes are first parsed as arguments in
# this shell, then as arguments in the adb shell, before finally being used as
# text inputs to the app. Hence, the "'wrapping'" on those commands.
# Using this approach there's no way to get any direct feedback from termux,
# so every time we run a command on it, we make sure it creates a unique *.probe file
# which is polled every 30 seconds together with the current output of the
# command in a *.log file. The contents of the probe file are used as a return code:
# 0 on success, some other number for errors (an empty file is basically the same as 0).
# Note that the return codes are text, not raw bytes.

# Additionally, we can use adb screenshot functionality to investigate issues
# when there is no feedback arriving from the android device.

this_repo="$(dirname "$(dirname -- "$(readlink -- "${0}")")")"
cache_dir_name="__rust_cache__"
dev_probe_dir=/sdcard
dev_home_dir=/data/data/com.termux/files/home

# This is a list of termux package mirrors approved to be used.
# The default mirror list contains entries that do not function properly anymore.
# To avoid failures due to broken mirrors, we use our own list.
# Choose only reliable mirrors here:
repo_url_list=(
    "deb https://packages-cf.termux.org/apt/termux-main/ stable main"
    "deb https://packages-cf.termux.dev/apt/termux-main/ stable main"
#    "deb https://grimler.se/termux/termux-main stable main"  # slow
    "deb https://ftp.fau.de/termux/termux-main stable main"
)
number_repo_urls=${#repo_url_list[@]}
repo_url_round_robin=$RANDOM

move_to_next_repo_url() {
    repo_url_round_robin=$(((repo_url_round_robin + 1) % number_repo_urls))
    echo "next round robin repo_url: $repo_url_round_robin"
}
move_to_next_repo_url # first call needed for modulo

get_current_repo_url() {
    echo "${repo_url_list[$repo_url_round_robin]}"
}

# dump some information about the runners system for debugging purposes:
echo "====== runner information ======"
echo "hostname: $(hostname)"
echo "uname -a: $(uname -a)"
echo "pwd: $(pwd)"
echo "\$*: $*"
echo "\$0: $0"
# shellcheck disable=SC2140
echo "\$(readlink -- "\$\{0\}"): $(readlink -- "${0}")"
echo "\$this_repo: $this_repo"
echo "readlink -f \$this_repo: $(readlink -f "$this_repo")"
this_repo=$(readlink -f "$this_repo")
echo "====== runner info end ========="

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
    adb shell input text \"exit\" && hit_enter && hit_enter
}

timestamp() {
  date +"%H%M%S%Z"
}

add_timestamp_to_lines() {
    while IFS= read -r line; do printf '[%s] %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$line"; done
}

# takes a screenshot with given name from the android device. Filename is prefixed with timestamp.
# screenshots are collected at the end of the github workflow and provided as download link.
take_screen_shot() {
    filename_prefix="$1"
    filename="$this_repo/output/$(timestamp)_${filename_prefix}_screen.png"
    echo "take screenshot: $filename"
    mkdir -p "$this_repo/output"
    adb exec-out screencap -p > "$filename"
}

get_app_user() {
    app="$1"
    app_user="$(adb shell dumpsys package $app | grep 'userId=' | cut -d= -f2 | sort -u)"
    if [[ -z "$app_user" ]]; then
        echo "Couldn't find user for app: $app">&2
        exit 1
    fi
    echo "$app_user"
}

termux_user() {
    if [[ -z "$TERMUX_USER" ]]; then
        TERMUX_USER="$(get_app_user com.termux)"
    fi
    echo "$TERMUX_USER"
}

launch_termux() {
    echo "launching termux"
    take_screen_shot "launch_termux_enter"

    adb shell input tap 120 380  # close potential dialog "System UI isn't responding" with "wait".
                                 # should not cause side effects when dialog is not there as there are
                                 # no relevant GUI elements at this position otherwise.

    if ! adb shell 'am start -n com.termux/.HomeActivity'; then
        echo "failed to launch termux"
        exit 1
    fi

    take_screen_shot "launch_termux_after_start_activity"

    # the emulator can sometimes be a little slow to launch the app
    loop_count=0
    while ! adb shell "dumpsys window windows" | \
            grep -E "imeInputTarget in display# 0 Window{[^}]+com.termux\/com\.termux\.HomeActivity}"
    do
        sleep 1
        loop_count=$((loop_count + 1))
        if [[ loop_count -ge 20 ]]; then
            break
        fi
    done

    take_screen_shot "launch_termux_after_wait_activity"

    touch_cmd() {
        adb shell input text "\"touch $dev_probe_dir/launch.probe\"" && hit_enter
        sleep 1
    }

    local timeout_start=120
    local timeout=$timeout_start
    touch_cmd
    while ! adb shell "ls $dev_probe_dir/launch.probe" 2>/dev/null
    do
        echo "waiting for launch.probe - ($timeout / $timeout_start seconds)"
        take_screen_shot "launch_termux_touch_probe"
        sleep 4
        touch_cmd

        timeout=$((timeout - 4))
        if [[ timeout -le 0 ]]; then
            take_screen_shot "error_launch_termux"
            echo "timeout waiting for termux to start up"
            return 1
        fi

    done
    echo "found launch.probe"
    take_screen_shot "launch_termux_found_probe"
    adb shell "rm $dev_probe_dir/launch.probe" && echo "removed launch.probe"
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
    log_file="$dev_probe_dir/${log_name}"
    log_read="${log_name}.read"
    echo 0 >"${log_read}"
    if [[ $debug -eq 1 ]]; then
        shell_command="'set -x; { ${command}; } &> ${log_file}; set +x'"
    else
        shell_command="'{ ${command}; } &> ${log_file}'"
    fi

    launch_termux || return

    take_screen_shot "run_termux_command_before_input_of_shell_command"

    # remove artificial quoting
    shell_command="${shell_command%\'}"
    shell_command="${shell_command#\'}"

    echo "Running command: ${command}"
    echo "Running shell-command: ${shell_command}"
    start=$(date +%s)
    adb_input_text_long "$shell_command" && sleep 1 && hit_enter
    # just for safety wait a little bit before polling for the probe and the log file
    sleep 1

    take_screen_shot "run_termux_command_after_input_of_shell_command"

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
            take_screen_shot "run_termux_command_maximum_tries_reached"
            return 1
        elif [[ try_fix -le 0 ]]; then
            retries=$((retries - 1))
            try_fix=3
            # Since there is no output, there is no way to know what is happening inside. See if
            # hitting the enter key solves the issue, sometimes the github runner is just a little
            # bit slow.
            echo "No output received. Trying to fix the issue ... (${retries} retries left)"
            take_screen_shot "run_termux_command_before_trying_to_fix"
            hit_enter
            sleep 1
            take_screen_shot "run_termux_command_after_trying_to_fix"
        fi

        sleep "$sleep_interval"
        timeout=$((timeout - sleep_interval))

        if [[ $timeout -le 0 ]]; then
            echo "Timeout reached running command. Aborting ..."
            take_screen_shot "run_termux_command_timeout_reached"
            return 1
        fi
    done
    end=$(date +%s)

    return_code=$(adb shell "cat $probe") || return_code=0
    adb shell "rm ${probe}"

    adb shell "cat $log_file" > "$log_name"
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

    take_screen_shot "run_termux_command_finished_normally"

    # shellcheck disable=SC2086
    return $return_code
}

init() {
    arch="$1"
    # shellcheck disable=SC2034
    api_level="$2"
    termux="$3"

    snapshot_name="${AVD_CACHE_KEY}"

    # shellcheck disable=SC2015
    wget -nv "https://github.com/termux/termux-app/releases/download/${termux}/termux-app_${termux}+github-debug_${arch}.apk" &&
        snapshot "termux-app_${termux}+github-debug_${arch}.apk" &&
        hash_rustc &&
        exit_termux &&
        adb -s emulator-5554 emu avd snapshot save "$snapshot_name" &&
        echo "Emulator image created. Name: $snapshot_name" || {
        pkill -9 qemu-system-x86_64
        return 1
    }
    pkill -9 qemu-system-x86_64 || true
}

reinit_ssh_connection() {
    setup_ssh_forwarding
    test_ssh_connection && return

    start_sshd_via_adb_shell && (
        test_ssh_connection && return
        generate_and_install_public_key && test_ssh_connection && return
    ) || (
        install_packages_via_adb_shell openssh openssl
        generate_and_install_public_key
        start_sshd_via_adb_shell
        test_ssh_connection && return
    ) || (
        echo "failed to setup ssh connection"
        return 1
    )
}

start_sshd_via_adb_shell() {
    echo "start sshd via adb shell"
    probe="$dev_probe_dir/sshd.probe"
    command="'sshd; echo \$? > $probe'"
    run_termux_command "$command" "$probe"
}

setup_ssh_forwarding() {
    echo "setup ssh forwarding"
    adb forward tcp:9022 tcp:8022
}

copy_file_or_dir_to_device_via_ssh() {
    scp -r "$1" "scp://$(termux_user)@127.0.0.1:9022/$2"
}

copy_file_or_dir_from_device_via_ssh() {
    scp -r "scp://$(termux_user)@127.0.0.1:9022/$1" "$2"
}

# runs the in args provided command on android side via ssh. forwards return code.
# adds a timestamp to every line to be able to see where delays are
run_command_via_ssh() {
    ssh -p 9022 "$(termux_user)@127.0.0.1" -o StrictHostKeyChecking=accept-new "$@" 2>&1 | add_timestamp_to_lines
    return "${PIPESTATUS[0]}"
}

test_ssh_connection() {
    run_command_via_ssh echo ssh connection is working
}

# takes a local (on runner side) script file and runs it via ssh on the virtual android device. forwards return code.
# adds a timestamp to every line to be able to see where delays are
run_script_file_via_ssh() {
    ssh -p 9022 "$(termux_user)@127.0.0.1" -o StrictHostKeyChecking=accept-new "bash -s" < "$1" 2>&1 | add_timestamp_to_lines
    return "${PIPESTATUS[0]}"
}

# Experiments showed that the adb shell input text functionality has a limitation for the input length.
# If input length is too big, the input is not fully provided to the android device.
# To avoid this, we divide large inputs into smaller chunks and put them one-by-one.
adb_input_text_long() {
    string=$1
    length=${#string}
    step=20
    p=0
    for ((i = 0; i < length-step; i = i + step)); do
        chunk="${string:i:$step}"
        adb shell input text "'$chunk'"
        p=$((i+step))
    done

    remaining="${string:p}"
    adb shell input text "'$remaining'"
}

generate_rsa_key_local() {
    yes "" | ssh-keygen -t rsa -b 4096 -C "Github Action" -N ""
}

install_rsa_pub() {

    run_command_via_ssh "echo hello" && return  # if this works, we are already fine. Skipping

    # remove old host identity:
    ssh-keygen -f ~/.ssh/known_hosts -R "[127.0.0.1]:9022"

    rsa_pub_key=$(cat ~/.ssh/id_rsa.pub)
    echo "====================================="
    echo "$rsa_pub_key"
    echo "====================================="

    adb shell input text \"echo \"

    adb_input_text_long "$rsa_pub_key"

    adb shell input text "\" >> ~/.ssh/authorized_keys\"" && hit_enter
    sleep 1
}

install_packages_via_adb_shell() {
    install_package_list="$*"

    install_packages_via_adb_shell_using_apt "$install_package_list"
    if [[ $? -ne 0 ]]; then
        echo "apt failed. Now try install with pkg as fallback."
        probe="$dev_probe_dir/pkg.probe"
        command="'mkdir -vp ~/.cargo/bin; yes | pkg install $install_package_list -y; echo \$? > $probe'"
        run_termux_command "$command" "$probe" || return 1
    fi

    return 0
}

# We use apt to install the packages as pkg doesn't respect any pre-defined mirror list.
# Its important to have a defined mirror list to avoid issues with broken mirrors.
install_packages_via_adb_shell_using_apt() {
    install_package_list="$*"

    repo_url=$(get_current_repo_url)
    move_to_next_repo_url
    echo "set apt repository url: $repo_url"
    probe="$dev_probe_dir/sourceslist.probe"
    command="'echo $repo_url | dd of=\$PREFIX/etc/apt/sources.list; echo \$? > $probe'"
    run_termux_command "$command" "$probe"

    probe="$dev_probe_dir/adb_install.probe"
    command="'mkdir -vp ~/.cargo/bin; apt update; yes | apt install $install_package_list -y; echo \$? > $probe'"
    run_termux_command "$command" "$probe"
}

install_packages_via_ssh_using_apt() {
    install_package_list="$*"

    repo_url=$(get_current_repo_url)
    move_to_next_repo_url
    echo "set apt repository url: $repo_url"
    run_command_via_ssh "echo $repo_url | dd of=\$PREFIX/etc/apt/sources.list"

    run_command_via_ssh "apt update; yes | apt install $install_package_list -y"
}

apt_upgrade_all_packages() {
    repo_url=$(get_current_repo_url)
    move_to_next_repo_url
    echo "set apt repository url: $repo_url"
    run_command_via_ssh "echo $repo_url | dd of=\$PREFIX/etc/apt/sources.list"

    run_command_via_ssh "apt update; yes | apt upgrade -y"
}

generate_and_install_public_key() {
    echo "generate local public private key pair"
    generate_rsa_key_local
    echo "install public key via 'adb shell input'"
    install_rsa_pub
    echo "installed ssh public key on device"
}

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

snapshot() {
    apk="$1"
    echo "Running snapshot"
    adb install -g "$apk"

    echo "Prepare and install system packages"

    reinit_ssh_connection || return 1

    apt_upgrade_all_packages

    install_packages_via_ssh_using_apt "rust binutils openssl tar mount-utils"

    echo "Read /proc/cpuinfo"
    run_command_via_ssh "cat /proc/cpuinfo"

    echo "Installing cargo-nextest"
    # We need to install nextest via cargo currently, since there is no pre-built binary for android x86
    # explicitly set CARGO_TARGET_DIR as otherwise a random generated tmp directory is used,
    # which prevents incremental build for the retries.
    command="export CARGO_TERM_COLOR=always && export CARGO_TARGET_DIR=\"cargo_install_target_dir\" && cargo install cargo-nextest"

    run_with_retry 3 run_command_via_ssh "$command"
    return_code=$?

    echo "Info about cargo and rust - via SSH Script"
    run_script_file_via_ssh "$this_repo/util/android-scripts/collect-info.sh"

    echo "Snapshot complete"
    # shellcheck disable=SC2086
    return $return_code
}

sync_host() {
    repo="$1"
    cache_home="${HOME}/${cache_dir_name}"
    cache_dest="$dev_home_dir/${cache_dir_name}"

    reinit_ssh_connection

    echo "Running sync host -> image: ${repo}"

    # run_command_via_ssh "mkdir $dev_home_dir/coreutils"

    copy_file_or_dir_to_device_via_ssh "$repo" "$dev_home_dir"
    [[ -e "$cache_home" ]] && copy_file_or_dir_to_device_via_ssh "$cache_home" "$cache_dest"

    echo "Finished sync host -> image: ${repo}"
}

sync_image() {
    repo="$1"
    cache_home="${HOME}/${cache_dir_name}"
    cache_dest="$dev_probe_dir/${cache_dir_name}"

    reinit_ssh_connection

    echo "Running sync image -> host: ${repo}"

    command="rm -rf $dev_probe_dir/coreutils ${cache_dest}; \
mkdir -p ${cache_dest}; \
cd ${cache_dest}; \
tar czf cargo.tgz -C ~/ .cargo; \
tar czf target.tgz -C ~/coreutils target; \
ls -la ${cache_dest}"
    run_command_via_ssh "$command" || return

    rm -rf "$cache_home"
    copy_file_or_dir_from_device_via_ssh "$cache_dest" "$cache_home" || return

    echo "Finished sync image -> host: ${repo}"
}

build() {
    echo "Running build"

    reinit_ssh_connection

    run_script_file_via_ssh "$this_repo/util/android-scripts/collect-info.sh"

    command="export CARGO_TERM_COLOR=always;
             export CARGO_INCREMENTAL=0; \
             cd ~/coreutils && cargo build --features feat_os_unix_android"
    run_with_retry 3 run_command_via_ssh "$command" || return

    echo "Finished build"
}

tests() {
    echo "Running tests"

    reinit_ssh_connection

    run_script_file_via_ssh "$this_repo/util/android-scripts/collect-info.sh"

    run_script_file_via_ssh "$this_repo/util/android-scripts/run-tests.sh" || return

    echo "Finished tests"
}

hash_rustc() {
    tmp_hash="__rustc_hash__.tmp"
    hash="__rustc_hash__"

    reinit_ssh_connection

    echo "Hashing rustc version: ${HOME}/${hash}"

    run_command_via_ssh "rustc -Vv" > rustc.log || return
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
