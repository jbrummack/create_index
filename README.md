Testbench for supporting Rust concurrency with USearch. 
Concurrent Index creation led to a ~10x shorter Index creation time using Rayon.

```rust
lines.flatten().enumerate().par_bridge().for_each(|(a, b)| { ... }) //Indexing 7177947 192-vectors takes 9 minutes
lines.flatten().enumerate().for_each(|(a, b)| { ... }) //Indexing 7177947 192-vectors takes 96 minutes
```
