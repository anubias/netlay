# NETwork reLAY

`netlay` is a command line Linux utility created for relaying TCP/UDP sockets between different machines. This is typically useful for bridging traffic across networks that are not routed together. This comes handy during development or quick maintenance or debugging sessions.

## Synopsis

Being a command line utility, the utility takes a few (optional) arguments.

```sh
netlay [OPTIONS]
```

OPTIONS:

```sh
-c, --config-file <CONFIG_FILE>     Path to the configuration file
-h, --help                          Print help
```

If no argument is provided, `netlay` will read the default configuration file located at `/etc/netlay.conf`. If you want to use a custom (or temporary) configuration file, please use the `-c` option.
