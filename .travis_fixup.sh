#!/usr/bin/env bash

if [ "$CI" != "true" ]; then
	exit 1
fi

case "$TRAVIS_RUST_VERSION" in
	"beta")
		skip=$(grep skip_on_beta Cargo.toml | cut -d\" -f 2)
		sed -i.org "/skip_on_beta/d" Cargo.toml
		;;
	"stable")
		skip=$(grep -E "skip_on_beta|skip_on_stable" Cargo.toml | cut -d\" -f 2)
		sed -i.org "/skip_on_beta/d" Cargo.toml
		sed -i.org "/skip_on_stable/d" Cargo.toml
		;;
esac

for x in $skip; do
	if [ -f tests/$x.rs ]; then
		mv tests/$x.rs tests/$x.rs.skip
	fi
done

