# liferaft

A rudimentary distributed kv database using the [raft](https://raft.github.io/) consensus algorithm.

This is mainly a learning project to understand how raft works and how to implement it.

### Usage

```console
$ cargo run -- --port 1234 --peers 1235

# In another terminal
$ cargo run -- --port 1235 --peers 1234
```
