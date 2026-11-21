# Security policy

## Reporting a vulnerability

Please **do not** open a public issue for security problems. Use GitHub's private reporting: the repository's **Security tab → "Report a vulnerability"**. We will acknowledge within a few days and keep you updated.

## What is in scope

SS2RS reads files and talks to peers that it cannot trust:

- **Parsers** (`.gro` archives, CTSEMETA and every asset format, mod files, settings) process untrusted data: malformed input must never cause memory unsafety, panics that take the process down, unbounded allocation or infinite loops.
- **Netcode** (server and client) processes untrusted packets from other players: it must validate every field, rate-limit, and never trust client-reported game state.
- **Mod/scripting** (future) must be sandboxed.

Reports about the original game's own bugs are out of scope.

## Supported versions

The project is pre-alpha; only the latest `main` is supported.
