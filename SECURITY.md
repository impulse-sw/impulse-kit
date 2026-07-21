# Security Policy

`impulse-kit` is a collection of full-stack Rust frameworks (Server Kit, Client
Kit, Static Server, and companions). We take security reports seriously.

## Reporting a vulnerability

**Please do not open public GitHub issues for security vulnerabilities.**

Report privately to **titoffklim@verbalautomation.tech** (or via GitHub Security
Advisories on this repository), including a description and impact, reproduction
steps, the affected version/commit, and any suggested fix. We aim to acknowledge
within a few business days and to remediate as quickly as severity warrants.
Please allow reasonable time for a fix before public disclosure.

## Hardening notes

- **Native binaries unwind on panic.** Release builds use `panic = "unwind"`;
  the aggressive `immediate-abort` strategy is confined to the `wasm-release`
  profile (see the README "Build profiles" section), so a reachable panic in a
  server (`iks`, `ring-server`, any Server Kit backend) does not abort the whole
  process.
- **`nightly` toolchain.** These crates require nightly; pin your toolchain and
  review nightly updates before rolling them into production.

## Supported versions

Security fixes target the latest `1.6.x` release and `unstable`.
