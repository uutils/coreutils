use std::panic;

pub fn install_sigpipe_hook() {
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
