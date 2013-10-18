build:
	rm -rf build
	mkdir build
	# run through the shell since make acting up on windows
	sh -c 'rustc --out-dir build/ false/false.rs'
	sh -c 'rustc --out-dir build/ printenv/printenv.rs'
	sh -c 'rustc --out-dir build/ true/true.rs'
	sh -c 'rustc --out-dir build/ yes/yes.rs'
	sh -c 'rustc --out-dir build/ cat/cat.rs'
	sh -c 'rustc --out-dir build/ whoami/whoami.rs'

.PHONY: build
