// spell-checker:ignore fname, tname, fpath, specfile, testfile, unspec, ifile, ofile, outfile, fullblock, urand, fileio, atoe, atoibm, behaviour, bmax, bremain, btotal, cflags, creat, ctable, ctty, datastructures, doesnt, etoa, fileout, fname, gnudd, iconvflags, nocache, noctty, noerror, nofollow, nolinks, nonblock, oconvflags, outfile, parseargs, rlen, rmax, rposition, rremain, rsofar, rstat, sigusr, sigval, wlen, wstat

use super::*;

mod block_unblock_tests;
mod conv_sync_tests;
mod conversion_tests;
mod sanity_tests;

use std::fs;
use std::io::prelude::*;
use std::io::BufReader;

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
    ( $ctable:expr ) =>
    {
        IConvFlags {
            ctable: $ctable,
            ..IConvFlags::default()
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
                            print_level: None,
                            count: None,
                            cflags: IConvFlags::default(),
                            iflags: IFlags::default(),
                        },
                        Output {
                            dst: File::create(format!("./test-resources/FAILED-{}.test", $test_name)).unwrap(),
                            obs: 512,
                            cflags: OConvFlags::default(),
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
            $o.dd_out($i).unwrap();

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
