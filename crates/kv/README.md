# struct Store
## Manage 
- `open(configurations)`
- `sync()`
## Crud
- `delete(key)`
- `put(key, value)`
- `get(key)`
- `list_keys()`
- `fold(&self, kv->bool)`
## Batched
- `new_batched()`
  - `delete(key)`
  - `put(key, value)`
  - `commit()`
## Iterator
- iter_options()
  - `begin()`
  - `rev()`
  - `with_prefix(prefix)`  
  - `make()`
## Storage
- `merge()`
- `blocking_copy_to()`




## Notes:
1. All integers are stored in **BIG endian** format.
2. `struct BatchedWrite` should actually hold an `Arc` to the store, not the reference.
   1. This is a design mistake...