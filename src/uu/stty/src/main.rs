// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// All output paths in uu_stty handle stdout write errors themselves and end
// with a newline (draining the line buffer), so the defensive exit-time flush
// is not necessary and would report write errors a second time.
uucore::bin!(uu_stty, no_flush);
