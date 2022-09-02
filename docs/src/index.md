{{#include logo.svg}}

<style>
    /* Make the logo a bit bigger and center */
    #logo {
        height: 200px;
        width: 100%;
    }

    /* This is necessary to get the <use> tags to obey the CSS styles below */
    g, polygon {
        fill: inherit;
        stroke: inherit;
    }

    /* Set the circle to the foreground color of the theme */
    #gear circle {
        stroke: var(--fg);
    }

    /* Set the stroke of polygons and the copies (via use) */
    #gear polygon,
    #gear use {
        fill: var(--fg);
        stroke: var(--fg);
    }
</style>

# uutils Coreutils Documentation

uutils is an attempt at writing universal (as in cross-platform) CLI utilities
in [Rust](https://www.rust-lang.org). It is available for Linux, Windows, Mac
and other platforms.

The API reference for `uucore`, the library of functions shared between various
utils, is hosted at [docs.rs](https://docs.rs/uucore/latest/uucore/).

uutils is licensed under the
[MIT License](https://github.com/uutils/coreutils/blob/main/LICENSE).

## Useful links

- [Releases](https://github.com/uutils/coreutils/releases)
- [Source Code](https://github.com/uutils/coreutils)
- [Issues](https://github.com/uutils/coreutils/issues)
- [Discord](https://discord.gg/wQVJbvJ)

> Note: This manual is automatically generated from the source code and is a
> work in progress.
