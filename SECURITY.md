# Security Policy

## Supported versions

Security fixes are applied to the latest `main` branch. There are no separate
maintenance branches; update to the latest release to receive fixes.

## Reporting a vulnerability

**Please do not report security vulnerabilities via public GitHub issues.**

Use GitHub's private vulnerability reporting instead:
**Report a vulnerability** tab on the [Security](https://github.com/takurot/ai-termchat/security/advisories/new)
page, or email the maintainer directly.

Include, if possible:

- A description of the issue and its impact
- Steps to reproduce or a proof of concept
- Affected versions / commits

You should receive an initial response within 72 hours. Please allow reasonable
time for a fix to be issued before any public disclosure.

## Security model (summary)

triadchat authenticates peers and encrypts transport on the LAN:

- **Peer identity** — each node generates an ed25519 keypair (`identity.key`).
  On first contact, peers exchange a signed `PeerIdentity` payload; the
  ed25519 public key fingerprint is bound to the endpoint.
- **Transport encryption** — after identity verification, peers run an
  x25519 key exchange (signed with the ed25519 key) and wrap payloads in
  `NetMessage::Secure` (ChaCha20Poly1305). Replay is mitigated by a
  monotonic send nonce + seen-nonce set.
- **Downgrade resistance** — once an endpoint is authenticated, plaintext
  payloads from that endpoint are rejected.
- **Skill execution** — only peers in `trusted_peers` may trigger skills;
  `risk: medium|high` skills require explicit confirmation.

See `docs/SPEC.md` §16 for the full network safety design and `src/secure.rs`
for the implementation. A security-review workflow is available via
`docs/PROMPT.md` and the `security-review` skill.
