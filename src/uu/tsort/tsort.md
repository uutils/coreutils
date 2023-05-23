# tsort

```
tsort [OPTIONS] FILE
```

Topological sort the strings in FILE.
Strings are defined as any sequence of tokens separated by whitespace (tab, space, or newline), ordering them based on dependencies in a directed acyclic graph (DAG). 
Useful for scheduling and determining execution order.
If FILE is not passed in, stdin is used instead.
