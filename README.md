# git-rs

> A simple git implementation in Rust

I'm building this to learn more about Rust and Git. Don't actually use this for
anything.

```bash
$ ./git init
$ echo "Hello world" > hello.txt
$ ./git add hello.txt
$ echo "Initial commit" | ./git commit
cdd69f086a8d8b0fbe93d91e48d53ce8750bd9c4
$ echo cdd69f086a8d8b0fbe93d91e48d53ce8750bd9c4 > .git/refs/heads/master
```

## Features to implement

- [ ] cached tree index extension
- [ ] rev parsing (in progress)
- [ ] fix adding for symlinks
- [ ] `git checkout` command
- [ ] `git diff` command
- [ ] reflog iteration
- [x] object cache creation
- [x] work tree index updating
- [x] tree parsing
- [x] `git add` command
- [x] commit creation
