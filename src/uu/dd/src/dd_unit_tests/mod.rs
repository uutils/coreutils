use super::*;

mod block_unblock_tests;
mod conv_sync_tests;
mod conversion_tests;
mod sanity_tests;

use std::fs;
use std::io::prelude::*;
use std::io::BufReader;

const DEFAULT_CFO: OConvFlags = OConvFlags {
    sparse: false,
    excl: false,
    nocreat: false,
    notrunc: false,
    fdatasync: false,
    fsync: false,
};

const DEFAULT_IFLAGS: IFlags = IFlags {
    cio: false,
    direct: false,
    directory: false,
    dsync: false,
    sync: false,
    nocache: false,
    nonblock: false,
    noatime: false,
    noctty: false,
    nofollow: false,
    nolinks: false,
    binary: false,
    text: false,
    fullblock: false,
    count_bytes: false,
    skip_bytes: false,
};

// const DEFAULT_OFLAGS: OFlags = OFlags {
//     append: false,
//     cio: false,
//     direct: false,
//     directory: false,
//     dsync: false,
//     sync: false,
//     nocache: false,
//     nonblock: false,
//     noatime: false,
//     noctty: false,
//     nofollow: false,
//     nolinks: false,
//     binary: false,
//     text: false,
//     seek_bytes: false,
// };

struct LazyReader<R: Read> {
    src: R,
}

impl<R: Read> Read for LazyReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let reduced = cmp::max(buf.len() / 2, 1);
        self.src.read(&mut buf[..reduced])
    }
}

#[macro_export]
macro_rules! icf (
    () =>
    {
        icf!(None)
    };
    ( $ctable:expr ) =>
    {
        IConvFlags {
            ctable: $ctable,
            block: None,
            unblock: None,
            swab: false,
            sync: None,
            noerror: false,
        }
    };
);

#[macro_export]
macro_rules! make_spec_test (
    ( $test_id:ident, $test_name:expr, $src:expr ) =>
    {
        // When spec not given, output should match input
        make_spec_test!($test_id, $test_name, $src, $src);
    };
    ( $test_id:ident, $test_name:expr, $src:expr, $spec:expr ) =>
    {
        make_spec_test!($test_id,
                        $test_name,
                        Input {
                            src: $src,
                            non_ascii: false,
                            ibs: 512,
                            xfer_stats: None,
                            count: None,
                            cflags: icf!(),
                            iflags: DEFAULT_IFLAGS,
                        },
                        Output {
                            dst: File::create(format!("./test-resources/FAILED-{}.test", $test_name)).unwrap(),
                            obs: 512,
                            cflags: DEFAULT_CFO,
                        },
                        $spec,
                        format!("./test-resources/FAILED-{}.test", $test_name)
        );
    };
    ( $test_id:ident, $test_name:expr, $i:expr, $o:expr, $spec:expr, $tmp_fname:expr ) =>
    {
        #[test]
        fn $test_id()
        {
            dd_fileout($i,$o).unwrap();

            let res = File::open($tmp_fname).unwrap();
            // Check test file isn't empty (unless spec file is too)
            assert_eq!(res.metadata().unwrap().len(), $spec.metadata().unwrap().len());

            let spec = BufReader::new($spec);
            let res = BufReader::new(res);

            // Check all bytes match
            for (b_res, b_spec) in res.bytes().zip(spec.bytes())
            {
                assert_eq!(b_res.unwrap(),
                           b_spec.unwrap());
            }

            fs::remove_file($tmp_fname).unwrap();
        }
    };
);
