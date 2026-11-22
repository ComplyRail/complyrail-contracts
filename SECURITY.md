# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| Phase 2 (v0.2.x) | ✅ |
| Phase 1 (v0.1.x) | ❌ |

## Reporting a Vulnerability

**Do not open a public GitHub issue for a security vulnerability.**

Please report vulnerabilities privately by emailing **security@complyrail.dev**.

Include:
- A description of the vulnerability and its potential impact.
- Steps to reproduce or a proof-of-concept.
- The contract version or commit hash affected.

You will receive an acknowledgement within **48 hours** and a status update within **7 days**.

If the vulnerability is confirmed we will coordinate a fix and disclose it publicly once a patched version is deployed. We credit researchers by name in the release notes unless you request anonymity.

## Scope

In scope:
- `src/lib.rs` and `src/types.rs` contract logic
- Authorization and access-control bypass
- Storage key collisions that could corrupt VASP or payment records
- Incorrect threshold evaluation that bypasses compliance gating

Out of scope:
- Off-chain IVMS101 message handling (outside this repository)
- Stellar network-layer issues (report to the Stellar Development Foundation)
- Issues in third-party dependencies (report upstream)
