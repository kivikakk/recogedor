# recogedor

Cursed IMAP forwarding service.


## todo

* harden against disconnects etc.
* ensure unsolicited EXISTS are picked up/check EXISTS races
* clean shutdown


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

TODO


### example

Forward new mail received on one Fastmail account with multiple aliases to two different local
accounts.

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
      (received-by "fox@foxden.net"))
    (append! "fox")
    (append! "wolf"))
  (flag! "Recogido")
"""
```


# legal

Copyright (c) 2023, Charlotte "charlottia", Asherah Connor.  
Licensed under the [Zero-Clause BSD License](LICENSE.txt).