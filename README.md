# type-sizes-rs

## Getting started

### 1. Generate type size information

```
cargo +nightly rustc -- -Zprint-type-sizes > types.txt
```

### 2. Analyze the output

```
cargo r -- <type-sizes-path>
```
