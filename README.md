# Redshell

For now, this is a half-built engine, a third of an MVP, and some 'concept art'.
You can try it out by installing from crates.io:

```sh
cargo install redshell
```

Or you can install it from source:

```sh
git clone https://github.com/nic-hartley/redshell.git
cd redshell
cargo install --path .
```

Once installed, you'll have a `redshell` binary, which lets you play the game, or you can run `redshell concept` to see the available concept art.

## Versioning

Redshell, despite being on crates.io, **does not really follow semver**.
The goal is that:

- Patch versions are bumped for small balance tweaks, bugfixes, etc.
- Minor versions are bumped when gameplay changes in significant ways or mods are likely to become incompatible
- Major versions are bumped with large gameplay changes (which will probably break mods, too)

So a mod written for, say, 1.2.3 should still be compatible with 1.2.4, though it may be unbalanced, but it might not load anymore on 1.3.0 and probably won't on 2.0.0.
Gameplay strategies will break more frequently, as they're much more sensitive to balance changes.
