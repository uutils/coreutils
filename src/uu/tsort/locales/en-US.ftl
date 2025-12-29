tsort-about = Topological sort the strings in FILE.
  Strings are defined as any sequence of tokens separated by whitespace (tab, space, or newline), ordering them based on dependencies in a directed acyclic graph (DAG).
  Useful for scheduling and determining execution order.
  If FILE is not passed in, stdin is used instead.
tsort-usage = tsort [OPTIONS] FILE
tsort-error-is-dir = read error: Is a directory
tsort-error-odd = input contains an odd number of tokens
tsort-error-loop = input contains a loop:
tsort-error-extra-operand = extra operand { $operand }
  Try '{ $util } --help' for more information.
tsort-error-at-least-one-input = at least one input
