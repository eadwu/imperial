# RORBind

Read-Only Recursive Bind Mount.

Does exactly what is says, creates a read only recursive bind mount that propagates `ro` to every submount as well.

Implementation is literally stripped from https://github.com/opencontainers/runtime-spec/pull/1090. A more concrete description of the problem can be found in linked issues such as https://github.com/docker/for-linux/issues/788.

## Requirements

- Linux >= 5.12 (`mount_setattr`)
