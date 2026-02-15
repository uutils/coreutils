#[cfg(any(target_os = "windows", target_os = "linux"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
uucore::bin!(uu_sha384sum);
