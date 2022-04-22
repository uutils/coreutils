# spell-checker:ignore termux keyevent sdcard binutils unmatch adb's dumpsys logcat pkill

# There are three shells: the host's, adb, and termux. Only adb lets us run
# commands directly on the emulated device, only termux provides a GNU
# environment on the emulated device (to e.g. run cargo). So we use adb to
# launch termux, then to send keystrokes to it while it's running.
# This means that the commands sent to termux are first parsed as arguments in
# this shell, then as arguments in the adb shell, before finally being used as
# text inputs to the app. Hence, the "'wrapping'" on those commands.
# There's no way to get any feedback from termux, so every time we run a
# command on it, we make sure it ends by creating a unique *.probe file at the
# end of the command. The contents of the file are used as a return code: 0 on
# success, some other number for errors (an empty file is basically the same as
# 0). Note that the return codes are text, not raw bytes.


this_repo="$(dirname $(dirname -- "$(readlink -- "${0}")"))"

help () {
    echo \
"Usage: $0 COMMAND [ARG]

where COMMAND is one of:
  snapshot APK  install APK and dependencies on an emulator to prep a snapshot
                (you can, but probably don't want to, run this for physical
                devices -- just set up termux and the dependencies yourself)
  sync [REPO]   push the repo at REPO to the device, deleting and restoring all
                symlinks (locally) in the process; by default, REPO is:
                $this_repo
  build         run \`cargo build --features feat_os_unix_android\` on the
                device, then pull the output as build.log
  tests         run \`cargo test --features feat_os_unix_android\` on the
                device, then pull the output as tests.log

If you have multiple devices, use the ANDROID_SERIAL environment variable to
specify which to connect to."
}

hit_enter() {
    adb shell input keyevent 66
}

launch_termux() {
    echo "launching termux"
    if ! adb shell 'am start -n com.termux/.HomeActivity' ; then
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

run_termux_command() {
    command="$1"  # text of the escaped command, including creating the probe!
    probe="$2"  # unique file that indicates the command is complete
    launch_termux
    adb shell input text "$command" && hit_enter
    while ! adb shell "ls $probe" 2>/dev/null; do echo "waiting for $probe"; sleep 30; done
    return_code=$(adb shell "cat $probe")
    adb shell "rm $probe"
    echo "return code: $return_code"
    return $return_code
}

snapshot () {
    apk="$1"
    echo "running snapshot"
    adb install -g "$apk"
    probe='/sdcard/pkg.probe'
    command="'yes | pkg install rust binutils openssl -y; touch $probe'"
    run_termux_command "$command" "$probe"
    echo "snapshot complete"
    adb shell input text "exit" && hit_enter && hit_enter
}

sync () {
    repo="$1"
    echo "running sync $1"
    # android doesn't allow symlinks on shared dirs, and adb can't selectively push files
    symlinks=$(find "$repo" -type l)
    # dash doesn't support process substitution :(
    echo $symlinks | sort >symlinks
    git -C "$repo" diff --name-status | cut -f 2 >modified
    modified_links=$(join symlinks modified)
    if [ ! -z "$modified_links" ]; then
        echo "You have modified symlinks. Either stash or commit them, then try again: $modified_links"
        exit 1
    fi
    if ! git ls-files --error-unmatch $symlinks >/dev/null; then
        echo "You have untracked symlinks. Either remove or commit them, then try again."
        exit 1
    fi
    rm $symlinks
    # adb's shell user only has access to shared dirs...
    adb push "$repo" /sdcard/coreutils
    git -C "$repo" checkout $symlinks
    # ...but shared dirs can't build, so move it home as termux
    probe='/sdcard/mv.probe'
    command="'cp -r /sdcard/coreutils ~/; touch $probe'"
    run_termux_command "$command" "$probe"
}

build () {
    probe='/sdcard/build.probe'
    command="'cd ~/coreutils && cargo build --features feat_os_unix_android 2>/sdcard/build.log; echo \$? >$probe'"
    echo "running build"
    run_termux_command "$command" "$probe"
    return_code=$?
    adb pull /sdcard/build.log .
    cat build.log
    return $return_code
}

tests () {
    probe='/sdcard/tests.probe'
    command="'cd ~/coreutils && cargo test --features feat_os_unix_android --no-fail-fast >/sdcard/tests.log 2>&1; echo \$? >$probe'"
    run_termux_command "$command" "$probe"
    return_code=$?
    adb pull /sdcard/tests.log .
    cat tests.log
    return $return_code
}

#adb logcat &
exit_code=0

if [ $# -eq 1 ]; then
    case "$1" in
        sync)     sync "$this_repo"; exit_code=$?;;
        build)    build; exit_code=$?;;
        tests)    tests; exit_code=$?;;
        *)        help;;
    esac
elif [ $# -eq 2 ]; then
    case "$1" in
        snapshot) snapshot "$2"; exit_code=$?;;
        sync)     sync "$2"; exit_code=$?;;
        *)        help; exit 1;;
    esac
else
    help
    exit_code=1
fi

#pkill adb
exit $exit_code
