use crate::common::util::*;
use std::env;

#[test]
fn test_users_noarg() {
    let (_, mut ucmd) = at_and_ucmd!();
    let result = ucmd.run();
    assert!(result.success);
}
#[test]
fn test_users_check_name() {
    let result = TestScenario::new(util_name!()).ucmd_keepenv().run();
    assert!(result.success);

    // Expectation: USER is often set
    let key = "USER";

    match env::var(key) {
        Err(e) => println!("Key {} isn't set. Found {}", &key, e),
        Ok(username) =>
        // Check if "users" contains the name of the user
        {
            println!("username found {}", &username);
            println!("result.stdout {}", &result.stdout);
            if !&result.stdout.is_empty() {
                assert!(result.stdout.contains(&username))
            }
        }
    }
}
