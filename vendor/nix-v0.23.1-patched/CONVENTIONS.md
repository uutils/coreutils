# Conventions

In order to achieve our goal of wrapping [libc][libc] code in idiomatic rust
constructs with minimal performance overhead, we follow the following
conventions.

Note that, thus far, not all the code follows these conventions and not all
conventions we try to follow have been documented here. If you find an instance
of either, feel free to remedy the flaw by opening a pull request with
appropriate changes or additions.

## Change Log

We follow the conventions laid out in [Keep A CHANGELOG][kacl].

[kacl]: https://github.com/olivierlacan/keep-a-changelog/tree/18adb5f5be7a898d046f6a4acb93e39dcf40c4ad

## libc constants, functions and structs

We do not define integer constants ourselves, but use or reexport them from the
[libc crate][libc].

We use the functions exported from [libc][libc] instead of writing our own
`extern` declarations.

We use the `struct` definitions from [libc][libc] internally instead of writing
our own. If we want to add methods to a libc type, we use the newtype pattern.
For example,

```rust
pub struct SigSet(libc::sigset_t);

impl SigSet {
    ...
}
```

When creating newtypes, we use Rust's `CamelCase` type naming convention.

## Bitflags

Many C functions have flags parameters that are combined from constants using
bitwise operations. We represent the types of these parameters by types defined
using our `libc_bitflags!` macro, which is a convenience wrapper around the
`bitflags!` macro from the [bitflags crate][bitflags] that brings in the
constant value from `libc`.

We name the type for a set of constants whose element's names start with `FOO_`
`FooFlags`.

For example,

```rust
libc_bitflags!{
    pub struct ProtFlags: libc::c_int {
        PROT_NONE;
        PROT_READ;
        PROT_WRITE;
        PROT_EXEC;
        #[cfg(any(target_os = "linux", target_os = "android"))]
        PROT_GROWSDOWN;
        #[cfg(any(target_os = "linux", target_os = "android"))]
        PROT_GROWSUP;
    }
}
```


## Enumerations

We represent sets of constants that are intended as mutually exclusive arguments
to parameters of functions by [enumerations][enum].


## Structures Initialized by libc Functions

Whenever we need to use a [libc][libc] function to properly initialize a
variable and said function allows us to use uninitialized memory, we use
[`std::mem::MaybeUninit`][std_MaybeUninit] when defining the variable. This
allows us to avoid the overhead incurred by zeroing or otherwise initializing
the variable.

[bitflags]: https://crates.io/crates/bitflags/
[enum]: https://doc.rust-lang.org/reference.html#enumerations
[libc]: https://crates.io/crates/libc/
[std_MaybeUninit]: https://doc.rust-lang.org/stable/std/mem/union.MaybeUninit.html
