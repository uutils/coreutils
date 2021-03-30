// This file is part of the uutils coreutils package.
//
// (c) kwantam <kwantam@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

use crate::common::util::*;
use std::time::SystemTime;

#[path = "../../src/uu/factor/sieve.rs"]
mod sieve;

extern crate conv;
extern crate rand;

use self::rand::distributions::{Distribution, Uniform};
use self::rand::{rngs::SmallRng, Rng, SeedableRng};
use self::sieve::Sieve;

const NUM_PRIMES: usize = 10000;
const NUM_TESTS: usize = 100;

#[test]
fn test_first_100000_integers() {
    extern crate sha1;

    let n_integers = 100_000;
    let mut instring = String::new();
    for i in 0..=n_integers {
        instring.push_str(&(format!("{} ", i))[..]);
    }

    println!("STDIN='{}'", instring);
    let result = new_ucmd!().pipe_in(instring.as_bytes()).succeeds();
    let stdout = result.stdout_str();

    // `seq 0 100000 | factor | sha1sum` => "4ed2d8403934fa1c76fe4b84c5d4b8850299c359"
    let hash_check = sha1::Sha1::from(stdout.as_bytes()).hexdigest();
    assert_eq!(hash_check, "4ed2d8403934fa1c76fe4b84c5d4b8850299c359");
}

#[test]
fn test_random() {
    use conv::prelude::*;

    let log_num_primes = f64::value_from(NUM_PRIMES).unwrap().log2().ceil();
    let primes = Sieve::primes().take(NUM_PRIMES).collect::<Vec<u64>>();

    let rng_seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("rng_seed={:?}", rng_seed);
    let mut rng = SmallRng::seed_from_u64(rng_seed);

    let mut rand_gt = move |min: u64| {
        let mut product = 1_u64;
        let mut factors = Vec::new();
        while product < min {
            // log distribution---higher probability for lower numbers
            let factor;
            loop {
                let next = rng.gen_range(0_f64, log_num_primes).exp2().floor() as usize;
                if next < NUM_PRIMES {
                    factor = primes[next];
                    break;
                }
            }
            let factor = factor;

            match product.checked_mul(factor) {
                Some(p) => {
                    product = p;
                    factors.push(factor);
                }
                None => break,
            };
        }

        factors.sort();
        (product, factors)
    };

    // build an input and expected output string from factor
    let mut instring = String::new();
    let mut outstring = String::new();
    for _ in 0..NUM_TESTS {
        let (product, factors) = rand_gt(1 << 63);
        instring.push_str(&(format!("{} ", product))[..]);

        outstring.push_str(&(format!("{}:", product))[..]);
        for factor in factors {
            outstring.push_str(&(format!(" {}", factor))[..]);
        }
        outstring.push_str("\n");
    }

    run(instring.as_bytes(), outstring.as_bytes());
}

#[test]
fn test_random_big() {
    let rng_seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("rng_seed={:?}", rng_seed);
    let mut rng = SmallRng::seed_from_u64(rng_seed);

    let bitrange_1 = Uniform::new(14_usize, 51);
    let mut rand_64 = move || {
        // first, choose a random number of bits for the first factor
        let f_bit_1 = bitrange_1.sample(&mut rng);
        // how many more bits do we need?
        let rem = 64 - f_bit_1;

        // we will have a number of additional factors equal to nfacts + 1
        // where nfacts is in [0, floor(rem/14) )  NOTE half-open interval
        // Each prime factor is at least 14 bits, hence floor(rem/14)
        let nfacts = Uniform::new(0_usize, rem / 14).sample(&mut rng);
        // we have to distribute extrabits among the (nfacts + 1) values
        let extrabits = rem - (nfacts + 1) * 14;
        // (remember, a Range is a half-open interval)
        let extrarange = Uniform::new(0_usize, extrabits + 1);

        // to generate an even split of this range, generate n-1 random elements
        // in the range, add the desired total value to the end, sort this list,
        // and then compute the sequential differences.
        let mut f_bits = Vec::new();
        for _ in 0..nfacts {
            f_bits.push(extrarange.sample(&mut rng));
        }
        f_bits.push(extrabits);
        f_bits.sort();

        // compute sequential differences here. We leave off the +14 bits
        // so we can just index PRIMES_BY_BITS
        let mut f_bits = f_bits
            .iter()
            .scan(0, |st, &x| {
                let ret = x - *st; // + 14 would give actual number of bits
                *st = x;
                Some(ret)
            })
            .collect::<Vec<usize>>();
        // finally, add f_bit_1 in there
        f_bits.push(f_bit_1 - 14); // index of f_bit_1 in PRIMES_BY_BITS
        let f_bits = f_bits;

        let mut nbits = 0;
        let mut product = 1_u64;
        let mut factors = Vec::new();
        for bit in f_bits {
            assert!(bit < 37);
            nbits += 14 + bit;
            let elm = Uniform::new(0, PRIMES_BY_BITS[bit].len()).sample(&mut rng);
            let factor = PRIMES_BY_BITS[bit][elm];
            factors.push(factor);
            product *= factor;
        }
        assert_eq!(nbits, 64);

        factors.sort();
        (product, factors)
    };

    let mut instring = String::new();
    let mut outstring = String::new();
    for _ in 0..NUM_TESTS {
        let (product, factors) = rand_64();
        instring.push_str(&(format!("{} ", product))[..]);

        outstring.push_str(&(format!("{}:", product))[..]);
        for factor in factors {
            outstring.push_str(&(format!(" {}", factor))[..]);
        }
        outstring.push_str("\n");
    }

    run(instring.as_bytes(), outstring.as_bytes());
}

#[test]
fn test_big_primes() {
    let mut instring = String::new();
    let mut outstring = String::new();
    for prime in PRIMES64 {
        instring.push_str(&(format!("{} ", prime))[..]);
        outstring.push_str(&(format!("{0}: {0}\n", prime))[..]);
    }

    run(instring.as_bytes(), outstring.as_bytes());
}

fn run(instring: &[u8], outstring: &[u8]) {
    println!("STDIN='{}'", String::from_utf8_lossy(instring));
    println!("STDOUT(expected)='{}'", String::from_utf8_lossy(outstring));
    // now run factor
    new_ucmd!()
        .pipe_in(instring)
        .run()
        .stdout_is(String::from_utf8(outstring.to_owned()).unwrap());
}

const PRIMES_BY_BITS: &'static [&'static [u64]] = &[
    PRIMES14, PRIMES15, PRIMES16, PRIMES17, PRIMES18, PRIMES19, PRIMES20, PRIMES21, PRIMES22,
    PRIMES23, PRIMES24, PRIMES25, PRIMES26, PRIMES27, PRIMES28, PRIMES29, PRIMES30, PRIMES31,
    PRIMES32, PRIMES33, PRIMES34, PRIMES35, PRIMES36, PRIMES37, PRIMES38, PRIMES39, PRIMES40,
    PRIMES41, PRIMES42, PRIMES43, PRIMES44, PRIMES45, PRIMES46, PRIMES47, PRIMES48, PRIMES49,
    PRIMES50,
];

const PRIMES64: &'static [u64] = &[
    18446744073709551557,
    18446744073709551533,
    18446744073709551521,
    18446744073709551437,
    18446744073709551427,
    18446744073709551359,
    18446744073709551337,
    18446744073709551293,
    18446744073709551263,
    18446744073709551253,
    18446744073709551191,
    18446744073709551163,
    18446744073709551113,
    18446744073709550873,
    18446744073709550791,
    18446744073709550773,
    18446744073709550771,
    18446744073709550719,
    18446744073709550717,
    18446744073709550681,
    18446744073709550671,
    18446744073709550593,
    18446744073709550591,
    18446744073709550539,
    18446744073709550537,
    18446744073709550381,
    18446744073709550341,
    18446744073709550293,
    18446744073709550237,
    18446744073709550147,
    18446744073709550141,
    18446744073709550129,
    18446744073709550111,
    18446744073709550099,
    18446744073709550047,
    18446744073709550033,
    18446744073709550009,
    18446744073709549951,
    18446744073709549861,
    18446744073709549817,
    18446744073709549811,
    18446744073709549777,
    18446744073709549757,
    18446744073709549733,
    18446744073709549667,
    18446744073709549621,
    18446744073709549613,
    18446744073709549583,
    18446744073709549571,
];

const PRIMES14: &'static [u64] = &[
    16381, 16369, 16363, 16361, 16349, 16339, 16333, 16319, 16301, 16273, 16267, 16253, 16249,
    16231, 16229, 16223, 16217, 16193, 16189, 16187, 16183, 16141, 16139, 16127, 16111, 16103,
    16097, 16091, 16087, 16073, 16069, 16067, 16063, 16061, 16057, 16033, 16007, 16001, 15991,
    15973, 15971, 15959, 15937, 15923, 15919, 15913, 15907, 15901, 15889, 15887, 15881, 15877,
    15859, 15823, 15817, 15809, 15803, 15797, 15791, 15787, 15773, 15767, 15761, 15749, 15739,
    15737, 15733, 15731, 15727, 15683, 15679, 15671, 15667, 15661, 15649, 15647, 15643, 15641,
    15629, 15619, 15607, 15601, 15583, 15581, 15569, 15559, 15551, 15541, 15527, 15511, 15497,
    15493, 15473, 15467, 15461, 15451, 15443, 15439, 15427, 15413, 15401, 15391, 15383, 15377,
    15373,
];

const PRIMES15: &'static [u64] = &[
    32749, 32719, 32717, 32713, 32707, 32693, 32687, 32653, 32647, 32633, 32621, 32611, 32609,
    32603, 32587, 32579, 32573, 32569, 32563, 32561, 32537, 32533, 32531, 32507, 32503, 32497,
    32491, 32479, 32467, 32443, 32441, 32429, 32423, 32413, 32411, 32401, 32381, 32377, 32371,
    32369, 32363, 32359, 32353, 32341, 32327, 32323, 32321, 32309, 32303, 32299, 32297, 32261,
    32257, 32251, 32237, 32233, 32213, 32203, 32191, 32189, 32183, 32173, 32159, 32143, 32141,
    32119, 32117, 32099, 32089, 32083, 32077, 32069, 32063, 32059, 32057, 32051, 32029, 32027,
    32009, 32003, 31991, 31981, 31973, 31963, 31957, 31907, 31891, 31883, 31873, 31859, 31849,
    31847, 31817, 31799, 31793, 31771, 31769, 31751,
];

const PRIMES16: &'static [u64] = &[
    65521, 65519, 65497, 65479, 65449, 65447, 65437, 65423, 65419, 65413, 65407, 65393, 65381,
    65371, 65357, 65353, 65327, 65323, 65309, 65293, 65287, 65269, 65267, 65257, 65239, 65213,
    65203, 65183, 65179, 65173, 65171, 65167, 65147, 65141, 65129, 65123, 65119, 65111, 65101,
    65099, 65089, 65071, 65063, 65053, 65033, 65029, 65027, 65011, 65003, 64997, 64969, 64951,
    64937, 64927, 64921, 64919, 64901, 64891, 64879, 64877, 64871, 64853, 64849, 64817, 64811,
    64793, 64783, 64781, 64763, 64747, 64717, 64709, 64693, 64679, 64667, 64663, 64661, 64633,
    64627, 64621, 64613, 64609, 64601, 64591, 64579, 64577, 64567, 64553,
];

const PRIMES17: &'static [u64] = &[
    131071, 131063, 131059, 131041, 131023, 131011, 131009, 130987, 130981, 130973, 130969, 130957,
    130927, 130873, 130859, 130843, 130841, 130829, 130817, 130811, 130807, 130787, 130783, 130769,
    130729, 130699, 130693, 130687, 130681, 130657, 130651, 130649, 130643, 130639, 130633, 130631,
    130621, 130619, 130589, 130579, 130553, 130547, 130531, 130523, 130517, 130513, 130489, 130483,
    130477, 130469, 130457, 130447, 130439, 130423, 130411, 130409, 130399, 130379, 130369, 130367,
    130363, 130349, 130343, 130337, 130307, 130303, 130279, 130267, 130261, 130259, 130253, 130241,
    130223, 130211, 130201, 130199, 130183, 130171, 130147, 130127, 130121, 130099, 130087, 130079,
    130073, 130069, 130057, 130051,
];

const PRIMES18: &'static [u64] = &[
    262139, 262133, 262127, 262121, 262111, 262109, 262103, 262079, 262069, 262051, 262049, 262027,
    262007, 261983, 261977, 261973, 261971, 261959, 261917, 261887, 261881, 261847, 261823, 261799,
    261791, 261787, 261773, 261761, 261757, 261739, 261721, 261713, 261707, 261697, 261673, 261643,
    261641, 261637, 261631, 261619, 261601, 261593, 261587, 261581, 261577, 261563, 261557, 261529,
    261523, 261509, 261467, 261463, 261451, 261439, 261433, 261431, 261427, 261407, 261389, 261379,
    261353, 261347, 261337, 261329, 261323, 261301, 261281, 261271, 261251, 261241, 261229, 261223,
    261169, 261167, 261127,
];

const PRIMES19: &'static [u64] = &[
    524287, 524269, 524261, 524257, 524243, 524231, 524221, 524219, 524203, 524201, 524197, 524189,
    524171, 524149, 524123, 524119, 524113, 524099, 524087, 524081, 524071, 524063, 524057, 524053,
    524047, 523997, 523987, 523969, 523949, 523937, 523927, 523907, 523903, 523877, 523867, 523847,
    523829, 523801, 523793, 523777, 523771, 523763, 523759, 523741, 523729, 523717, 523681, 523673,
    523669, 523667, 523657, 523639, 523637, 523631, 523603, 523597, 523577, 523573, 523571, 523553,
    523543, 523541, 523519, 523511, 523493, 523489, 523487, 523463, 523459, 523433, 523427, 523417,
    523403, 523387, 523357, 523351, 523349, 523333, 523307, 523297,
];

const PRIMES20: &'static [u64] = &[
    1048573, 1048571, 1048559, 1048549, 1048517, 1048507, 1048447, 1048433, 1048423, 1048391,
    1048387, 1048367, 1048361, 1048357, 1048343, 1048309, 1048291, 1048273, 1048261, 1048219,
    1048217, 1048213, 1048193, 1048189, 1048139, 1048129, 1048127, 1048123, 1048063, 1048051,
    1048049, 1048043, 1048027, 1048013, 1048009, 1048007, 1047997, 1047989, 1047979, 1047971,
    1047961, 1047941, 1047929, 1047923, 1047887, 1047883, 1047881, 1047859, 1047841, 1047833,
    1047821, 1047779, 1047773, 1047763, 1047751, 1047737, 1047721, 1047713, 1047703, 1047701,
    1047691, 1047689, 1047671, 1047667, 1047653, 1047649, 1047647, 1047589, 1047587, 1047559,
];

const PRIMES21: &'static [u64] = &[
    2097143, 2097133, 2097131, 2097097, 2097091, 2097083, 2097047, 2097041, 2097031, 2097023,
    2097013, 2096993, 2096987, 2096971, 2096959, 2096957, 2096947, 2096923, 2096911, 2096909,
    2096893, 2096881, 2096873, 2096867, 2096851, 2096837, 2096807, 2096791, 2096789, 2096777,
    2096761, 2096741, 2096737, 2096713, 2096693, 2096687, 2096681, 2096639, 2096629, 2096621,
    2096599, 2096597, 2096569, 2096539, 2096533, 2096483, 2096449, 2096431, 2096429, 2096411,
    2096407, 2096401, 2096399, 2096377, 2096357, 2096291, 2096273, 2096261, 2096233, 2096231,
    2096221, 2096209, 2096191, 2096183, 2096147,
];

const PRIMES22: &'static [u64] = &[
    4194301, 4194287, 4194277, 4194271, 4194247, 4194217, 4194199, 4194191, 4194187, 4194181,
    4194173, 4194167, 4194143, 4194137, 4194131, 4194107, 4194103, 4194023, 4194011, 4194007,
    4193977, 4193971, 4193963, 4193957, 4193939, 4193929, 4193909, 4193869, 4193807, 4193803,
    4193801, 4193789, 4193759, 4193753, 4193743, 4193701, 4193663, 4193633, 4193573, 4193569,
    4193551, 4193549, 4193531, 4193513, 4193507, 4193459, 4193447, 4193443, 4193417, 4193411,
    4193393, 4193389, 4193381, 4193377, 4193369, 4193359, 4193353, 4193327, 4193309, 4193303,
    4193297,
];

const PRIMES23: &'static [u64] = &[
    8388593, 8388587, 8388581, 8388571, 8388547, 8388539, 8388473, 8388461, 8388451, 8388449,
    8388439, 8388427, 8388421, 8388409, 8388377, 8388371, 8388319, 8388301, 8388287, 8388283,
    8388277, 8388239, 8388209, 8388187, 8388113, 8388109, 8388091, 8388071, 8388059, 8388019,
    8388013, 8387999, 8387993, 8387959, 8387957, 8387947, 8387933, 8387921, 8387917, 8387891,
    8387879, 8387867, 8387861, 8387857, 8387839, 8387831, 8387809, 8387807, 8387741, 8387737,
    8387723, 8387707, 8387671, 8387611, 8387609, 8387591,
];

const PRIMES24: &'static [u64] = &[
    16777213, 16777199, 16777183, 16777153, 16777141, 16777139, 16777127, 16777121, 16777099,
    16777049, 16777027, 16776989, 16776973, 16776971, 16776967, 16776961, 16776941, 16776937,
    16776931, 16776919, 16776901, 16776899, 16776869, 16776857, 16776839, 16776833, 16776817,
    16776763, 16776731, 16776719, 16776713, 16776691, 16776689, 16776679, 16776659, 16776631,
    16776623, 16776619, 16776607, 16776593, 16776581, 16776547, 16776521, 16776491, 16776481,
    16776469, 16776451, 16776401, 16776391, 16776379, 16776371, 16776367, 16776343, 16776337,
    16776317, 16776313, 16776289, 16776217, 16776211,
];

const PRIMES25: &'static [u64] = &[
    33554393, 33554383, 33554371, 33554347, 33554341, 33554317, 33554291, 33554273, 33554267,
    33554249, 33554239, 33554221, 33554201, 33554167, 33554159, 33554137, 33554123, 33554093,
    33554083, 33554077, 33554051, 33554021, 33554011, 33554009, 33553999, 33553991, 33553969,
    33553967, 33553909, 33553901, 33553879, 33553837, 33553799, 33553787, 33553771, 33553769,
    33553759, 33553747, 33553739, 33553727, 33553697, 33553693, 33553679, 33553661, 33553657,
    33553651, 33553649, 33553633, 33553613, 33553607, 33553577, 33553549, 33553547, 33553537,
    33553519, 33553517, 33553511, 33553489, 33553463, 33553451, 33553417,
];

const PRIMES26: &'static [u64] = &[
    67108859, 67108837, 67108819, 67108777, 67108763, 67108757, 67108753, 67108747, 67108739,
    67108729, 67108721, 67108709, 67108693, 67108669, 67108667, 67108661, 67108649, 67108633,
    67108597, 67108579, 67108529, 67108511, 67108507, 67108493, 67108471, 67108463, 67108453,
    67108439, 67108387, 67108373, 67108369, 67108351, 67108331, 67108313, 67108303, 67108289,
    67108271, 67108219, 67108207, 67108201, 67108199, 67108187, 67108183, 67108177, 67108127,
    67108109, 67108081, 67108049, 67108039, 67108037, 67108033, 67108009, 67108007, 67108003,
    67107983, 67107977, 67107967, 67107941, 67107919, 67107913, 67107883, 67107881, 67107871,
    67107863,
];

const PRIMES27: &'static [u64] = &[
    134217689, 134217649, 134217617, 134217613, 134217593, 134217541, 134217529, 134217509,
    134217497, 134217493, 134217487, 134217467, 134217439, 134217437, 134217409, 134217403,
    134217401, 134217367, 134217361, 134217353, 134217323, 134217301, 134217277, 134217257,
    134217247, 134217221, 134217199, 134217173, 134217163, 134217157, 134217131, 134217103,
    134217089, 134217079, 134217049, 134217047, 134217043, 134217001, 134216987, 134216947,
    134216939, 134216933, 134216911, 134216899, 134216881, 134216869, 134216867, 134216861,
    134216837, 134216827, 134216807, 134216801, 134216791, 134216783, 134216777, 134216759,
    134216737, 134216729,
];

const PRIMES28: &'static [u64] = &[
    268435399, 268435367, 268435361, 268435337, 268435331, 268435313, 268435291, 268435273,
    268435243, 268435183, 268435171, 268435157, 268435147, 268435133, 268435129, 268435121,
    268435109, 268435091, 268435067, 268435043, 268435039, 268435033, 268435019, 268435009,
    268435007, 268434997, 268434979, 268434977, 268434961, 268434949, 268434941, 268434937,
    268434857, 268434841, 268434827, 268434821, 268434787, 268434781, 268434779, 268434773,
    268434731, 268434721, 268434713, 268434707, 268434703, 268434697, 268434659, 268434623,
    268434619, 268434581, 268434577, 268434563, 268434557, 268434547, 268434511, 268434499,
    268434479, 268434461,
];

const PRIMES29: &'static [u64] = &[
    536870909, 536870879, 536870869, 536870849, 536870839, 536870837, 536870819, 536870813,
    536870791, 536870779, 536870767, 536870743, 536870729, 536870723, 536870717, 536870701,
    536870683, 536870657, 536870641, 536870627, 536870611, 536870603, 536870599, 536870573,
    536870569, 536870563, 536870561, 536870513, 536870501, 536870497, 536870473, 536870401,
    536870363, 536870317, 536870303, 536870297, 536870273, 536870267, 536870239, 536870233,
    536870219, 536870171, 536870167, 536870153, 536870123, 536870063, 536870057, 536870041,
    536870027, 536869999, 536869951, 536869943, 536869937, 536869919, 536869901, 536869891,
];

const PRIMES30: &'static [u64] = &[
    1073741789, 1073741783, 1073741741, 1073741723, 1073741719, 1073741717, 1073741689, 1073741671,
    1073741663, 1073741651, 1073741621, 1073741567, 1073741561, 1073741527, 1073741503, 1073741477,
    1073741467, 1073741441, 1073741419, 1073741399, 1073741387, 1073741381, 1073741371, 1073741329,
    1073741311, 1073741309, 1073741287, 1073741237, 1073741213, 1073741197, 1073741189, 1073741173,
    1073741101, 1073741077, 1073741047, 1073740963, 1073740951, 1073740933, 1073740909, 1073740879,
    1073740853, 1073740847, 1073740819, 1073740807,
];

const PRIMES31: &'static [u64] = &[
    2147483647, 2147483629, 2147483587, 2147483579, 2147483563, 2147483549, 2147483543, 2147483497,
    2147483489, 2147483477, 2147483423, 2147483399, 2147483353, 2147483323, 2147483269, 2147483249,
    2147483237, 2147483179, 2147483171, 2147483137, 2147483123, 2147483077, 2147483069, 2147483059,
    2147483053, 2147483033, 2147483029, 2147482951, 2147482949, 2147482943, 2147482937, 2147482921,
    2147482877, 2147482873, 2147482867, 2147482859, 2147482819, 2147482817, 2147482811, 2147482801,
    2147482763, 2147482739, 2147482697, 2147482693, 2147482681, 2147482663, 2147482661,
];

const PRIMES32: &'static [u64] = &[
    4294967291, 4294967279, 4294967231, 4294967197, 4294967189, 4294967161, 4294967143, 4294967111,
    4294967087, 4294967029, 4294966997, 4294966981, 4294966943, 4294966927, 4294966909, 4294966877,
    4294966829, 4294966813, 4294966769, 4294966667, 4294966661, 4294966657, 4294966651, 4294966639,
    4294966619, 4294966591, 4294966583, 4294966553, 4294966477, 4294966447, 4294966441, 4294966427,
    4294966373, 4294966367, 4294966337, 4294966297,
];

const PRIMES33: &'static [u64] = &[
    8589934583, 8589934567, 8589934543, 8589934513, 8589934487, 8589934307, 8589934291, 8589934289,
    8589934271, 8589934237, 8589934211, 8589934207, 8589934201, 8589934187, 8589934151, 8589934141,
    8589934139, 8589934117, 8589934103, 8589934099, 8589934091, 8589934069, 8589934049, 8589934027,
    8589934007, 8589933973, 8589933971, 8589933967, 8589933931, 8589933917, 8589933907, 8589933853,
    8589933827, 8589933823, 8589933787, 8589933773, 8589933733, 8589933731, 8589933721, 8589933683,
    8589933647, 8589933641, 8589933637, 8589933631, 8589933629, 8589933619, 8589933601, 8589933581,
];

const PRIMES34: &'static [u64] = &[
    17179869143,
    17179869107,
    17179869071,
    17179869053,
    17179869041,
    17179869019,
    17179868999,
    17179868977,
    17179868957,
    17179868903,
    17179868899,
    17179868887,
    17179868879,
    17179868873,
    17179868869,
    17179868861,
    17179868843,
    17179868833,
    17179868809,
    17179868807,
    17179868777,
    17179868759,
    17179868729,
    17179868711,
    17179868683,
    17179868681,
    17179868597,
    17179868549,
    17179868543,
    17179868521,
    17179868513,
    17179868479,
    17179868443,
    17179868437,
    17179868429,
    17179868383,
    17179868369,
    17179868357,
    17179868353,
    17179868351,
    17179868333,
    17179868317,
    17179868309,
    17179868297,
    17179868287,
    17179868249,
    17179868243,
    17179868183,
];

const PRIMES35: &'static [u64] = &[
    34359738337,
    34359738319,
    34359738307,
    34359738299,
    34359738289,
    34359738247,
    34359738227,
    34359738121,
    34359738059,
    34359738043,
    34359738011,
    34359737917,
    34359737869,
    34359737849,
    34359737837,
    34359737821,
    34359737813,
    34359737791,
    34359737777,
    34359737771,
    34359737717,
    34359737591,
    34359737567,
    34359737549,
    34359737519,
    34359737497,
    34359737479,
    34359737407,
    34359737393,
    34359737371,
];

const PRIMES36: &'static [u64] = &[
    68719476731,
    68719476719,
    68719476713,
    68719476671,
    68719476619,
    68719476599,
    68719476577,
    68719476563,
    68719476547,
    68719476503,
    68719476493,
    68719476479,
    68719476433,
    68719476407,
    68719476391,
    68719476389,
    68719476377,
    68719476361,
    68719476323,
    68719476307,
    68719476281,
    68719476271,
    68719476257,
    68719476247,
    68719476209,
    68719476197,
    68719476181,
    68719476169,
    68719476157,
    68719476149,
    68719476109,
    68719476053,
    68719476047,
    68719476019,
    68719475977,
    68719475947,
    68719475933,
    68719475911,
    68719475893,
    68719475879,
    68719475837,
    68719475827,
    68719475809,
    68719475791,
    68719475779,
    68719475771,
    68719475767,
    68719475731,
    68719475729,
];

const PRIMES37: &'static [u64] = &[
    137438953447,
    137438953441,
    137438953427,
    137438953403,
    137438953349,
    137438953331,
    137438953273,
    137438953271,
    137438953121,
    137438953097,
    137438953037,
    137438953009,
    137438952953,
    137438952901,
    137438952887,
    137438952869,
    137438952853,
    137438952731,
    137438952683,
    137438952611,
    137438952529,
    137438952503,
    137438952491,
];

const PRIMES38: &'static [u64] = &[
    274877906899,
    274877906857,
    274877906837,
    274877906813,
    274877906791,
    274877906759,
    274877906753,
    274877906717,
    274877906713,
    274877906687,
    274877906647,
    274877906629,
    274877906627,
    274877906573,
    274877906543,
    274877906491,
    274877906477,
    274877906473,
    274877906431,
    274877906419,
    274877906341,
    274877906333,
    274877906327,
    274877906321,
    274877906309,
    274877906267,
    274877906243,
    274877906213,
    274877906209,
    274877906203,
    274877906179,
    274877906167,
    274877906119,
    274877906063,
    274877906053,
    274877906021,
    274877905931,
];

const PRIMES39: &'static [u64] = &[
    549755813881,
    549755813869,
    549755813821,
    549755813797,
    549755813753,
    549755813723,
    549755813669,
    549755813657,
    549755813647,
    549755813587,
    549755813561,
    549755813513,
    549755813507,
    549755813461,
    549755813417,
    549755813401,
    549755813371,
    549755813359,
    549755813357,
    549755813351,
    549755813339,
    549755813317,
    549755813311,
    549755813281,
    549755813239,
    549755813231,
    549755813213,
    549755813207,
    549755813197,
    549755813183,
    549755813161,
    549755813149,
    549755813147,
    549755813143,
    549755813141,
    549755813059,
    549755813027,
    549755813003,
    549755812951,
    549755812937,
    549755812933,
    549755812889,
    549755812867,
];

const PRIMES40: &'static [u64] = &[
    1099511627689,
    1099511627609,
    1099511627581,
    1099511627573,
    1099511627563,
    1099511627491,
    1099511627483,
    1099511627477,
    1099511627387,
    1099511627339,
    1099511627321,
    1099511627309,
    1099511627297,
    1099511627293,
    1099511627261,
    1099511627213,
    1099511627191,
    1099511627177,
    1099511627173,
    1099511627143,
    1099511627089,
    1099511626987,
    1099511626949,
    1099511626937,
    1099511626793,
    1099511626781,
    1099511626771,
];

const PRIMES41: &'static [u64] = &[
    2199023255531,
    2199023255521,
    2199023255497,
    2199023255489,
    2199023255479,
    2199023255477,
    2199023255461,
    2199023255441,
    2199023255419,
    2199023255413,
    2199023255357,
    2199023255327,
    2199023255291,
    2199023255279,
    2199023255267,
    2199023255243,
    2199023255203,
    2199023255171,
    2199023255137,
    2199023255101,
    2199023255087,
    2199023255081,
    2199023255069,
    2199023255027,
    2199023255021,
    2199023254979,
    2199023254933,
    2199023254913,
    2199023254907,
    2199023254903,
    2199023254843,
    2199023254787,
    2199023254699,
    2199023254693,
    2199023254657,
    2199023254567,
];

const PRIMES42: &'static [u64] = &[
    4398046511093,
    4398046511087,
    4398046511071,
    4398046511051,
    4398046511039,
    4398046510961,
    4398046510943,
    4398046510939,
    4398046510889,
    4398046510877,
    4398046510829,
    4398046510787,
    4398046510771,
    4398046510751,
    4398046510733,
    4398046510721,
    4398046510643,
    4398046510639,
    4398046510597,
    4398046510577,
    4398046510547,
    4398046510531,
    4398046510463,
    4398046510397,
    4398046510391,
    4398046510379,
    4398046510357,
    4398046510331,
    4398046510327,
    4398046510313,
    4398046510283,
    4398046510279,
    4398046510217,
    4398046510141,
    4398046510133,
    4398046510103,
    4398046510093,
];

const PRIMES43: &'static [u64] = &[
    8796093022151,
    8796093022141,
    8796093022091,
    8796093022033,
    8796093021953,
    8796093021941,
    8796093021917,
    8796093021899,
    8796093021889,
    8796093021839,
    8796093021803,
    8796093021791,
    8796093021769,
    8796093021763,
    8796093021743,
    8796093021671,
    8796093021607,
    8796093021587,
    8796093021533,
    8796093021523,
    8796093021517,
    8796093021493,
    8796093021467,
    8796093021461,
    8796093021449,
    8796093021409,
    8796093021407,
    8796093021371,
    8796093021347,
    8796093021337,
    8796093021281,
    8796093021269,
];

const PRIMES44: &'static [u64] = &[
    17592186044399,
    17592186044299,
    17592186044297,
    17592186044287,
    17592186044273,
    17592186044267,
    17592186044129,
    17592186044089,
    17592186044057,
    17592186044039,
    17592186043987,
    17592186043921,
    17592186043889,
    17592186043877,
    17592186043841,
    17592186043829,
    17592186043819,
    17592186043813,
    17592186043807,
    17592186043741,
    17592186043693,
    17592186043667,
    17592186043631,
    17592186043591,
    17592186043577,
    17592186043547,
    17592186043483,
    17592186043451,
    17592186043433,
    17592186043409,
];

const PRIMES45: &'static [u64] = &[
    35184372088777,
    35184372088763,
    35184372088751,
    35184372088739,
    35184372088711,
    35184372088699,
    35184372088693,
    35184372088673,
    35184372088639,
    35184372088603,
    35184372088571,
    35184372088517,
    35184372088493,
    35184372088471,
    35184372088403,
    35184372088391,
    35184372088379,
    35184372088363,
    35184372088321,
    35184372088319,
    35184372088279,
    35184372088259,
    35184372088249,
    35184372088241,
    35184372088223,
    35184372088183,
    35184372088097,
    35184372088081,
    35184372088079,
    35184372088051,
    35184372088043,
    35184372088039,
    35184372087937,
    35184372087929,
    35184372087923,
    35184372087881,
    35184372087877,
    35184372087869,
];

const PRIMES46: &'static [u64] = &[
    70368744177643,
    70368744177607,
    70368744177601,
    70368744177587,
    70368744177497,
    70368744177467,
    70368744177427,
    70368744177377,
    70368744177359,
    70368744177353,
    70368744177331,
    70368744177289,
    70368744177283,
    70368744177271,
    70368744177257,
    70368744177227,
    70368744177167,
    70368744177113,
    70368744177029,
    70368744176959,
    70368744176921,
    70368744176909,
    70368744176879,
    70368744176867,
    70368744176833,
    70368744176827,
    70368744176807,
    70368744176779,
    70368744176777,
    70368744176729,
    70368744176719,
    70368744176711,
];

const PRIMES47: &'static [u64] = &[
    140737488355213,
    140737488355201,
    140737488355181,
    140737488355049,
    140737488355031,
    140737488354989,
    140737488354893,
    140737488354787,
    140737488354709,
    140737488354679,
    140737488354613,
    140737488354557,
    140737488354511,
    140737488354431,
    140737488354413,
    140737488354409,
    140737488354373,
    140737488354347,
    140737488354329,
];

const PRIMES48: &'static [u64] = &[
    281474976710597,
    281474976710591,
    281474976710567,
    281474976710563,
    281474976710509,
    281474976710491,
    281474976710467,
    281474976710423,
    281474976710413,
    281474976710399,
    281474976710339,
    281474976710327,
    281474976710287,
    281474976710197,
    281474976710143,
    281474976710131,
    281474976710129,
    281474976710107,
    281474976710089,
    281474976710087,
    281474976710029,
    281474976709987,
    281474976709891,
    281474976709859,
    281474976709831,
    281474976709757,
    281474976709741,
    281474976709711,
    281474976709649,
    281474976709637,
];

const PRIMES49: &'static [u64] = &[
    562949953421231,
    562949953421201,
    562949953421189,
    562949953421173,
    562949953421131,
    562949953421111,
    562949953421099,
    562949953421047,
    562949953421029,
    562949953420973,
    562949953420871,
    562949953420867,
    562949953420837,
    562949953420793,
    562949953420747,
    562949953420741,
    562949953420733,
    562949953420727,
    562949953420609,
    562949953420571,
    562949953420559,
    562949953420553,
    562949953420523,
    562949953420507,
    562949953420457,
    562949953420403,
    562949953420373,
    562949953420369,
    562949953420343,
    562949953420303,
    562949953420297,
];

const PRIMES50: &'static [u64] = &[
    1125899906842597,
    1125899906842589,
    1125899906842573,
    1125899906842553,
    1125899906842511,
    1125899906842507,
    1125899906842493,
    1125899906842463,
    1125899906842429,
    1125899906842391,
    1125899906842357,
    1125899906842283,
    1125899906842273,
    1125899906842247,
    1125899906842201,
    1125899906842177,
    1125899906842079,
    1125899906842033,
    1125899906842021,
    1125899906842013,
    1125899906841973,
    1125899906841971,
    1125899906841959,
    1125899906841949,
    1125899906841943,
    1125899906841917,
    1125899906841901,
    1125899906841883,
    1125899906841859,
    1125899906841811,
    1125899906841803,
    1125899906841751,
    1125899906841713,
    1125899906841673,
    1125899906841653,
    1125899906841623,
    1125899906841613,
];
