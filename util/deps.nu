# This is a script to analyze the dependencies of this project.
# It is a replacement of / complement to
#
#  - cargo tree (used by this script)
#  - cargo deps
#  - cargo deny
#
# The idea is that by calling all_dep_info, you get a table of all dependencies
# in Cargo.lock, with a few additional columns based on some other tools.
# Currently, these tools are
#
#  - cargo tree
#  - the crates.io API
#
# The most useful columns in the table are:
#
#  - `name`: the name of the crate.
#  - `version`: the version of the crate.
#  - `num_versions`: the number of versions in Cargo.lock.
#  - `normal_dep`: whether the crate is a normal dependency.
#  - `build_dep`: whether the crate is a build dependency.
#  - `dev_dep`: whether the crate is a dev dependency.
#  - `organization`: the GitHub/GitLab organization or user of the repository of the crate.
#  - `repository_name`: the name of the repository the crate is in. The format is "{owner}/{repo}".
#  - `dependencies`: direct dependencies of the crate (in the format of Cargo.lock).
#
# To use this script, start Nushell (tested only on version 0.82.0), import the library and
# call `all_dep_info`:
#
# ```
# > nu
# > use util/deps.nu
# > let dep = (deps all_dep_info)
# ```
#
# Then you can perform analysis. For example, to group the dependencies by organization:
#
# ```
# > $dep | group-by organization
# ```
#
# Or to find all crates with multiple versions (like cargo deny):
# ```
# > $dep | where num_versions > 1
# ```
#
# Ideas to expand this:
#
#  - Figure out the whole dependency graph
#  - Figure out which platforms and which features enable which crates
#  - Figure out which utils require which crates
#  - Count the number of crates on different platforms
#  - Add license information
#  - Add functions to perform common analyses
#  - Add info from cargo bloat
#  - Add MSRV info
#  - Add up-to-date info (the necessary info is there, it just needs to be derived)
#  - Check the number of owners/contributors
#  - Make a webpage to more easily explore the data

# Read the names output by cargo tree
export def read_tree_names [edge_kind: string, features: list<string>]: any -> table<> {
    cargo tree --edges $edge_kind --features ($features | str join ",")
        | parse -r "(?P<name>[a-zA-Z0-9_-]+) v(?P<version>[0-9.]+)"
}

# Read the crates.io info for a list of crates names
def read_crates_io [names: list<string>] -> any -> table<> {
    let total = $names | length
    $names | enumerate | par-each {|name|
        print $"($name.index)/($total): ($name.item)"
        http get $"https://crates.io/api/v1/crates/($name.item)" | get crate
    }
}

# Add column for a dependency type
def add_dep_type [dep_type: string, features: list<string>]: table<> -> table<> {
    let input_table = $in
    let table = read_tree_names $dep_type $features
    $input_table | insert $"($dep_type)_dep" {|outer|
        $table | any {|inner|
            $inner.name == $outer.name and $inner.version == $outer.version
        }
    }
}

export def all_dep_info [] {
    let features = [unix, feat_selinux]

    let lock = open Cargo.lock | from toml | get package
    
    $lock
    # Add number of versions
    | join ($lock | group-by name | transpose | update column1 { length } | rename name num_versions) name
    # Add dependency types
    | add_dep_type normal $features
    | add_dep_type build $features
    | add_dep_type dev $features
    | insert used {|x| $x.normal_dep or $x.build_dep or $x.dev_dep}
    # Add crates.io info
    | join (read_crates_io ($lock.name | uniq)) name
    # Add GH org or user info
    # The organization is an indicator that crates should be treated as one dependency.
    # However, there are also unrelated projects by a single organization, so it's not
    # clear.
    | insert organization {|x|
        let repository = $x.repository?
        if ($repository == null) { "" } else {
            $repository | url parse | get path | path split | get 1
        }
    }
    # Add repository (truncate everything after repo name)
    # If we get a url like
    #  https://github.com/uutils/coreutils/tree/src/uu/ls
    # we only keep
    #  uutils/coreutils
    # The idea is that crates in the same repo definitely belong to the same project and should
    # be treated as one dependency.
    | insert repository_name {|x|
        let repository = $x.repository?
        if ($repository == null) { '' } else {
            $repository
            | url parse
            | get path
            | path split
            | select 1 2
            | path join
        }
    }
}
