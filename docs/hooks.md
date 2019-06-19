Hooks
-----

snowpatch supports running hooks before posting test results.

Hooks can be used to do things like send notification emails, or modify result URLs to point to externally-visible servers.

Hooks are specified by the `hooks` option in the `[patchwork]` section of the configuration, which is a list of hook scripts to be executed in order.

The hook script will be given the JSON representing the test result being posted to Patchwork, via stdin.

If a hook doesn't output anything, snowpatch will use the input test result unmodified. If a hook outputs modified JSON on stdout, snowpatch will use the modified JSON.

Example hooks can be found in `examples/hooks`.

 TODO: Do we want hooks to be specified globally or per project? Do we need to pass project name or other details on command line?