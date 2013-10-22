struct options {
    ignore_env: bool,
    null: bool,
    unsets: ~[~str],
    sets: ~[(~str, ~str)],
    program: ~[~str]
}

fn usage(prog: &str) {
    println!("Usage: {:s} [OPTION]... [-] [NAME=VALUE]... [COMMAND [ARG]...]", prog);
    println!("Sets each NAME as VALUE in the environment, then run COMMAND\n");
    println!("Possible options are:");
    println!("  -i, --ignore-environment  starts with an empty environment");
    println!("  -0, --null end each line with a 0 byte instead of a \\\\n\n");
}

fn version() {
    println!("env (Rust Coreutils) 1.0");
}

fn print_env(null: bool) {
    println!("env!")
}

fn main() {
    let args = std::os::args();
    let prog = args[0].as_slice();

    // to handle arguments the same way than GNU env, we can't use getopts
    // and have to do this:

    let mut opts = ~options {
        ignore_env: false,
        null: false,
        unsets: ~[],
        sets: ~[],
        program: ~[]
    };

    let mut wait_cmd = false;
    let mut iter = args.iter();
    iter.next(); // skip program

    for opt in iter {
        if wait_cmd {
            // we still accept NAME=VAL here but not other options
            let mut sp = opt.splitn_iter('=', 1);
            let name = sp.next();
            let value = sp.next();

            match (name, value) {
                (Some(n), Some(v)) => { 
                    opts.sets.push((n.into_owned(), v.into_owned())); 
                }
                _ => {
                    // read the program now
                    opts.program.push(opt.to_owned());
                    break;
                }
            }
        }

        else {
            if opt.starts_with("--") {
                match *opt {
                    ~"--help" => { usage(prog); return }
                    ~"--version" => { version(); return }

                    ~"--ignore-environment" => {
                        opts.ignore_env = true; 
                    }

                    ~"--null" => {
                        opts.null = true;
                    }
                            
                    _ => {
                        println!("{:s}: invalid option \"{:s}\"", prog, *opt);
                        println!("Type \"{:s} --help\" for detailed informations", prog);
                        return
                    }
                }
            }

            else {
                match *opt {
                    ~"-" => {
                        // implies -i and stop parsing opts
                        wait_cmd = true;
                        opts.ignore_env = true;
                    }

                    _ => {
                        // is it a NAME=VALUE like opt ?
                        let mut sp = opt.splitn_iter('=', 1);
                        let name = sp.next();
                        let value = sp.next();

                        match (name, value) {
                            (Some(n), Some(v)) => { 
                                // yes
                                opts.sets.push((n.into_owned(), v.into_owned())); 
                                wait_cmd = true;
                            }
                            // no, its a program-like opt
                            _ => {
                                opts.program.push(opt.to_owned());
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // read program arguments now
    for opt in iter {
        opts.program.push(opt.to_owned());
    }

    let env = std::os::env();

    if opts.ignore_env {
        for &(ref name, _) in env.iter() {
            std::os::unsetenv(name.as_slice())
        }
    }

    for &(ref name, ref val) in opts.sets.iter() {
        std::os::setenv(name.as_slice(), val.as_slice())
    }

    match opts.program {
        [ref prog, ..args] => { 
            let status = std::run::process_status(prog.as_slice(), args);
            std::os::set_exit_status(status)
        }
        [] => { print_env(opts.null); }
    }
}
