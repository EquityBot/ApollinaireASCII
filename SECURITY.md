# Security

ApollinaireASCII reads untrusted files (images, GIFs, FIGlet fonts and JSON projects) and the browser editor runs them through WebAssembly, so a parsing flaw matters.

Please report a vulnerability privately through this repository's private vulnerability reporting (Security tab, 'Report a vulnerability', or <https://github.com/yungSNAT/ApollinaireASCII/security/advisories/new>), not in a public issue. Include the version or commit, the input that triggers it and what you saw.

In scope: crashes, hangs or unbounded memory use from a crafted input; output outside printable ASCII; script injection through exported HTML players; and anything that makes the editor send data off the device. Known dependency alerts and why they are unreachable are listed in [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md).
