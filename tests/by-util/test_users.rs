use crate::common::util::*;
use std::env;

#[test]
fn test_users_noarg() {
    new_ucmd!().succeeds();
}
#[test]
fn test_users_check_name() {
    let result = TestScenario::new(util_name!()).ucmd_keepenv().succeeds();

    // Expectation: USER is often set
    let key = "USER";

    match env::var(key) {
        Err(e) => println!("Key {} isn't set. Found {}", &key, e),
        Ok(username) =>
        // Check if "users" contains the name of the user
        {
            println!("username found {}", &username);
            // println!("result.stdout {}", &result.stdout);
            if !result.stdout_str().is_empty() {
                result.stdout_contains(&username);
            }
        }
    }
}
