Use `el apply` with a script to make all five changes in one transaction:
1. add module `src/extra.rs`
2. add `pub fn one() -> u32` returning `1`
3. add `pub fn two() -> u32` returning `2`
4. add `pub fn three() -> u32` returning `3`
5. add `pub const LIMIT: u32 = 5`
The crate must compile and the journal must show one apply entry.
