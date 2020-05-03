Usage
=====

```
bcachefs-mount 0.1.0
Mount a bcachefs filesystem by its UUID

USAGE:
    bcachefs-mount [OPTIONS] <uuid> <mountpoint>

FLAGS:
    -h, --help       
            Prints help information

    -V, --version    
            Prints version information


OPTIONS:
    -o <options>                 
            Mount options [default: ]

    -p, --password <password>    
            Where the password would be loaded from.
            
            Possible values are: "fail" - don't ask for password, fail if filesystem is encrypted; "wait" - wait for
            password to become available before mounting; "ask" -  prompt the user for password; [default: fail]

ARGS:
    <uuid>          
            External UUID of the bcachefs filesystem

    <mountpoint>    
            Where the filesystem should be mounted
```

Caveats
=======

* `--password ask` is not yet implemented, but you can use `--password wait`, and load the key with `bcachefs unlock`.

Build
=====

```sh
$ git submodule update --init --recursive
$ cargo build --release
```

Binary will be built in `target/release/bcachefs-mount`

Dependencies:

* rust
* blkid
* uuid
* liburcu
* libsodium
* zlib
* liblz4
* libzstd
* libkeyutils
