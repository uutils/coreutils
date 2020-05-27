use std::panic;

//## SIGPIPE handling background/discussions ...
//* `uutils` ~ <https://github.com/uutils/coreutils/issues/374> , <https://github.com/uutils/coreutils/pull/1106>
//* rust and `rg` ~ <https://github.com/rust-lang/rust/issues/62569> , <https://github.com/BurntSushi/ripgrep/issues/200> , <https://github.com/crev-dev/cargo-crev/issues/287>

pub fn mute_sigpipe_panic() {
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if let Some(res) = info.payload().downcast_ref::<String>() {
            if res.contains("Broken pipe") {
                return;
            }
        }
        hook(info)
    }));
}
