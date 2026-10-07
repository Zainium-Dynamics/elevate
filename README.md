# elevate

A sudo and doas replacement in Rust, built for Zainium OS. It started as a
fork of [sudo-rs](https://github.com/memorysafety/sudo-rs) and has since gotten
its own policy handling and a "core protector" that refuses destructive
commands (`rm`, `mv`, `cp`, ...) aimed at the protected OS layers, even for
root. It authenticates through
[elevate-pam](https://github.com/Zainium-Dynamics/elevate-privilege).

## Binaries

- `elevate` replaces `sudo`. Filters and forwards the caller's environment,
  authenticates through PAM, logs to `LOG_AUTHPRIV`.
- `elev` replaces `doas`/`su`. Lighter: skips most of the environment handling
  `elevate` does.
- `vielev` replaces `visudo`. Edits the policy file under a lock and checks
  the syntax before saving, so a bad edit doesn't replace a working config.

```sh
elevate systemctl restart nginx
elevate -u postgres psql
vielev
```

A wrong password prints `Sorry, incorrect password, try again.` and asks
again, up to three times, then exits with `3 incorrect password attempts`.

## Building

```sh
cargo build --release
```

By default `.cargo/config.toml` builds a static musl binary. For a normal host
build use `--target x86_64-unknown-linux-gnu`. The binary has no link-time PAM
dependency: it loads `libelevate_pam.so.0` with `dlopen` at runtime (see
`src/pam/dynload.rs`). Install the binaries setuid root:

```sh
install -m 4755 target/release/elevate /bin/elevate
install -m 4755 target/release/elev    /bin/elev
install -m 0755 target/release/vielev  /sbin/vielev
```

More in [ZAINIUM_DEPLOY_README.md](ZAINIUM_DEPLOY_README.md) and
[MUSL_BUILD_README.md](MUSL_BUILD_README.md).

## Paths

Everything is looked up under a prefix, `/overlayer/syshub` by default
(Zainium's layout). To change it, put a `[paths]` table in a config file.
elevate uses the first of these it finds:

1. `$ELEVATE_CONFIG`
2. `/etc/elevate/config.toml`
3. `/overlayer/syshub/etc/elevate/config.toml`
4. `./elevate.toml`

```toml
[paths]
prefix = "/opt/zai"
# etc_dir        = "<prefix>/etc"
# bindir         = "<prefix>/bin"
# sbindir        = "<prefix>/bin"
# libdir         = "<prefix>/lib"       # where libelevate_pam.so is looked for
# elevators_file = "<prefix>/etc/elevators/elevate.toml"
```

Anything you leave out is worked out from `prefix`.

## License

MIT OR Apache-2.0, see [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE). Upstream notices are in
[NOTICE.md](NOTICE.md) and [COPYRIGHT](COPYRIGHT).
