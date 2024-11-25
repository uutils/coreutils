# Benchmarking `tsort`
<!-- spell-checker:ignore (words) randint tsort DAG uu_tsort GNU  -->
Much of what makes `tsort` fast is the efficiency of its algorithm and implementation for topological sorting. 
Our implementation of `tsort` also outputs a cycle whenever such ordering does not exist, just like GNU  `tsort`. 

## Strategies

To test `tsort`'s performance for its nominal use case, we need to test it with a DAG. One of the worst cases is when all nodes are just representing a succession of independent steps. 
We should also test cycle detection for good measure. 

### Random acyclic graph (DAG)

This will output a DAG composed of 1 million pairs of edges between nodes numbered from 0 to 10,000, ensuring that the graph is acyclic by always assigning the edge with the smallest id to the node with the highest one.

```python
import random

N = 10000

for i in range(100*N):
    a = random.randint(0, N)
    b = random.randint(0, N)
    print(f"{min(a, b)} {max(a, b)}")
```

### Random graph with cycles

The following will output a graph with multiples edges, it also allows some degree of tuning to test different cases. 

```python
import random

# Parameters for the graph
num_nodes = 100  
num_edges = 150  
cycle_percentage = 0.10  
max_cycle_size = 6  

num_cycles = int(num_edges * cycle_percentage)

for _ in range(num_edges - num_cycles):
    a = random.randint(0, num_nodes)
    b = random.randint(0, num_nodes)
    print(f"{a} {b}")


for _ in range(num_cycles):
    cycle_size = random.randint(3, max_cycle_size)
    cycle_nodes = random.sample(range(num_nodes), cycle_size)
    for i in range(cycle_size):
        print(f"{cycle_nodes[i]} {cycle_nodes[(i + 1) % cycle_size]}")
```

## Running Benchmarks
The above scripts will output the generated graphs to the standard output. They can therefore be used directly as tests. In order to run a Benchmark, the output should be redirected to a file. 
Use [`hyperfine`](https://github.com/sharkdp/hyperfine) to compare the performance of different `tsort` versions. For example, you can compare the performance of GNU `tsort` and another implementation with the following command:

```sh
hyperfine 'tsort random_graph.txt' 'uu_tsort random_graph.txt'
```

## Note

Benchmark results from the above scripts are fuzzy and change from run to run unless a seed is set.
