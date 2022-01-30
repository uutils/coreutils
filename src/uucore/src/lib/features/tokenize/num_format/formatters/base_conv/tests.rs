// spell-checker:ignore (ToDO) arrnum mult

#[cfg(test)]
use super::*;

#[test]
fn test_arrnum_int_mult() {
    // (in base 10) 12 * 4 = 48
    let factor: Vec<u8> = vec![1, 2];
    let base_num = 10;
    let base_ten_int_fact: u8 = 4;
    let should_output: Vec<u8> = vec![4, 8];

    let product = arrnum_int_mult(&factor, base_num, base_ten_int_fact);
    assert!(product == should_output);
}

#[test]
fn test_arrnum_int_non_base_10() {
    // (in base 3)
    // 5 * 4 = 20
    let factor: Vec<u8> = vec![1, 2];
    let base_num = 3;
    let base_ten_int_fact: u8 = 4;
    let should_output: Vec<u8> = vec![2, 0, 2];

    let product = arrnum_int_mult(&factor, base_num, base_ten_int_fact);
    assert!(product == should_output);
}

#[test]
fn test_arrnum_int_div_short_circuit() {
    // (
    let arrnum: Vec<u8> = vec![5, 5, 5, 5, 0];
    let base_num = 10;
    let base_ten_int_divisor: u8 = 41;
    let remainder_passed_in = Remainder {
        position: 1,
        replace: vec![1, 3],
        arr_num: &arrnum,
    };

    // the "replace" should mean the number being divided
    // is 1350, the first time you can get 41 to go into
    // 1350, its at 135, where you can get a quotient of
    // 3 and a remainder of 12;

    let quotient_should_be: u8 = 3;
    let remainder_position_should_be: usize = 3;
    let remainder_replace_should_be = vec![1, 2];

    let result = arrnum_int_div_step(&remainder_passed_in, base_num, base_ten_int_divisor, false);
    assert!(quotient_should_be == result.quotient);
    assert!(remainder_position_should_be == result.remainder.position);
    assert!(remainder_replace_should_be == result.remainder.replace);
}
