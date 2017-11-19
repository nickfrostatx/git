# git-rs

> A simple git implementation in Rust

I'm building this to learn more about Rust and Git. Don't actually use this for
anything.

```bash
$ echo "Hello world" > hello.txt
$ ./git add hello.txt
$ ./git write-tree
a50b30eb6b223aef893c367a0b93e9a5b21f155f
$ echo "Initial commit" | ./git commit a50b30eb6b223aef893c367a0b93e9a5b21f155f
cdd69f086a8d8b0fbe93d91e48d53ce8750bd9c4
$ echo cdd69f086a8d8b0fbe93d91e48d53ce8750bd9c4 > .git/refs/heads/master
```

## TODO

* Implement the cached tree index extension
