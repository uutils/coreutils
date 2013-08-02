build:
	rm -rf build
	mkdir build
	# run through the shell since make acting up on windows
	sh -c 'rustc --out-dir build/ false/false.rs'
	sh -c 'rustc --out-dir build/ printenv/printenv.rs'
	sh -c 'rustc --out-dir build/ true/true.rs'
	sh -c 'rustc --out-dir build/ yes/yes.rs'

.PHONY: build
