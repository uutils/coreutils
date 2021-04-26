expected = <<-'END'
x
y
z
i
j
k
END

errors = 0

while true
	x = `touch x y z i j k && cargo run -p uu_ls --  -t --time=birth x y z i j k 2>/dev/null`
	if x != expected
		errors += 1
		puts `cargo run -p uu_ls --  -t --time=birth x y z i j k -l --time-style=full-iso 2>/dev/null`
		puts "error #{errors}"
	end
	`rm x y z i j k`
end
