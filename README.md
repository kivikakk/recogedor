# recogedor

Cursed IMAP forwarding service.


## todo

* harden against disconnects etc.
* ensure unsolicited EXISTS are picked up/check EXISTS races
* clean shutdown
* dry-run should evaluate append targets (and everything that implies)
* compile script to native code/WASM/something. You Know You Want To.


## install

Add the repository to your flake inputs:

```nix
inputs.recogedor.url = github:charlottia/recogedor;
```

Add the NixOS module exposed by the flake:

```nix
nixosSystem {
  # ...
  modules = [
    # ...
    recogedor.nixosModules.${system}.default
  ];
}
```

Configure your system to use recogedor:

```nix
services.recogedor = {
  enable = true;
  settings = lib.importTOML ./recogedor.toml;
};
```


## config

Configure a source and one or more destination mailboxes.  `imap` is currently the only supported
`type`.  TLS is always used.  `host` is used for SNI.  The `ip` can be specified manually.

The process script is a Lisp.  I'm terribly sorry.  One or more sexps define the action to be taken
on each mail item.  **You must implement your own idempotency method.**  Recogedor will rescan the
entire source INBOX on startup and every time it's woken from IDLE.

Builtin names are unadorned symbols. Flags, recipient patterns, and destinations are strings.
Statement and condition forms are cons cells where the car identifies the builtin.

The following statement forms are defined:

* `(if C T E)` -- evaluate the condition C and execute statement T if true, E otherwise.  E may be
  omitted.
* `(halt!)` -- stop processing this mail item.
* `(append! D)` -- append this mail item to destination D.
* `(flag! F)` -- set the flag F on the mail item.

The following condition forms are defined:

* `(or C*)` -- true if any C is true.
* `(flagged F)` -- true if the mail item has the flag F.
* `(received-by R)` -- true if any recipient in the mail item's envelope matches the recipient
  pattern R.

Recipient patterns consist of an optional user part, an optional plus part, and an optional host
part.  At least one part must be specified.  A recipient matches a recipient pattern if all parts
defined in the pattern are case-insensitive equal to the corresponding parts of the recipient.  The
`+` character is not considered part of the plus part.

The syntax is roughly defined as follows: `(user)?(+plus)?@(host)?`.  Note that a recipient pattern
always contains an `@` symbol.  Examples follow:

* `abc@def.com` -- matches `abc@def.com` and `ABC+X@DEF.COM`.
* `+@def.com` -- matches `abc@def.com` and `xyz@def.com`.  Does not match `a+b@def.com`.
* `+debug@` -- matches `hello+debug@world.com` and `x+debug@i.net`.
* `support@` -- matches `support@nyonk` and `support+123@shomk`.


### example

Forward new mail received on one Fastmail account with multiple aliases to two different local
accounts.  Mail is flagged to avoid double handling.  Mail is flagged *after* appending to fail
"safe" -- an untimely power outage will result in double appending, not zero appending.

```toml
[src]
type = "imap"
host = "imap.fastmail.com"
port = 993
user = "fox@den.com"
pass = "abc123"

[dest.fox]
type = "imap"
host = "my.mx.com"
ip = "127.0.0.1"
port = 993
user = "fox@den.com"
pass = "ghi789"

[dest.wolf]
type = "imap"
host = "my.mx.com"
ip = "127.0.0.1"
port = 993
user = "wolf@den.com"
pass = "jkl012"

[process]
script = """
  (if (flagged "Recogido") (halt!))
  (if
    (or
      (received-by "fox@den.com")
      (received-by "s.fox@foxden.net"))
    (append! "fox")
    (append! "wolf"))
  (flag! "Recogido")
"""
```


## development

* Unit tests? Just design and write it correctly the first time.


# legal

Copyright (c) 2023, Charlotte "charlottia", Asherah Connor.  
Licensed under the [Zero-Clause BSD License](LICENSE.txt).