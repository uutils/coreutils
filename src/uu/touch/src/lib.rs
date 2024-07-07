// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

pub mod touch;
pub mod uu_args;
pub use uu_args::options;
pub use uu_args::uu_app;

pub use touch::uumain;
