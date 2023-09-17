# recogedor

IMAP forwarding service.


## todo

* configure forwarding criteria and action on forward
  * ignore mail with a given flag
  * add a flag on forward
  * forward to which endpoint based on what?
* harden against disconnects etc.
* ensure unsolicited EXISTS are picked up
* one "job" per process


## install

Add the repository to your flake inputs:

```nix
inputs.recogedor.url = github:kivikakk/recogedor;
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
services.kivikakk.recogedor = {
  enable = true;
  settings = lib.importTOML ./recogedor.toml;
};
```


## config

To be documented once the shape of the program is better figured out.


### example

Forward new mail received on two different Fastmail accounts to two different local accounts.

```toml
[jobs]
fox = { src = "fm-fox", dest = "my-fox" }
wolf = { src = "fm-wolf", dest = "my-wolf" }

[endpoints.fm-fox]
type = "imap"
host = "imap.fastmail.com"
port = 993
user = "fox@den.com"
pass = "abc123"

[endpoints.fm-wolf]
type = "imap"
host = "imap.fastmail.com"
port = 993
user = "wolf@den.com"
pass = "def456"

[endpoints.my-fox]
type = "imap"
host = "my.mx.com"
ip = "127.0.0.1"
port = 993
user = "fox@den.com"
pass = "ghi789"

[endpoints.my-wolf]
type = "imap"
host = "my.mx.com"
ip = "127.0.0.1"
port = 993
user = "wolf@den.com"
pass = "jkl012"
```


# legal

Copyright (c) 2023, Asherah Connor, Charlotte "charlottia".  
Licensed under the [Zero-Clause BSD License](LICENSE.txt).