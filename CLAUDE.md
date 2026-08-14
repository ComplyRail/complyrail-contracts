# ComplyRail - Project Documentation

ComplyRail is an open-source, on-chain compliance layer for Stellar anchors enforcing FATF Travel Rule requirements at the settlement layer.

## Project Structure

Three repositories under ComplyRail organization:

1. **complyrail-contracts** - Soroban smart contracts (Rust)
   - Location: `/workspaces/complyrail-contracts`
   - Phase 1-2 complete: Data structures, admin functions, VASP registry, payment management, attestation
   - Build: `cargo test --lib` or `docker run --rm -v /workspaces/complyrail-contracts:/workspace complyrail-build:latest`

2. **complyrail-app** - Compliance dashboard (Next.js/TypeScript)
   - Location: `/workspaces/complyrail-app`
   - Phase 1 baseline: Project structure, dependencies, CI/CD
   - Status: Ready for Phase 4 implementation

3. **complyrail-sdk** - Integration SDK (TypeScript/npm)
   - Location: `/workspaces/complyrail-sdk`
   - Phase 3 complete: IVMS101 builder, crypto utilities, types, examples
   - Build: `npm run build` (outputs ESM + CJS)
   - Test: `npm test` (14 tests passing)

## Current Status

### Phase 1 ✅ - Setup
- Three repositories created with CI/CD pipelines
- GitHub Actions configured for all repos
- Apache 2.0 licenses and base documentation

### Phase 2 ✅ - Contracts MVP
**File:** `/workspaces/complyrail-contracts/src/`
- `lib.rs`: All contract functions (register_vasp, submit_payment, submit_attestation, release_payment, reject_payment)
- `types.rs`: VaspStatus, VaspEntry, PaymentStatus, PaymentRecord, ThresholdConfig

**Key Features:**
- Multi-admin support (not single-admin EOA)
- Per-asset, per-jurisdiction thresholds
- Automatic payment release for below-threshold amounts
- IVMS101 message hash attestation for above-threshold
- Full event audit trail
- 8 unit tests covering all flows

### Phase 3 ✅ - SDK Implementation  
**File:** `/workspaces/complyrail-sdk/src/`

**Implemented:**
- `types.ts` - TypeScript interfaces matching Soroban contract (VaspEntry, PaymentRecord, IVMS101Message)
- `client.ts` - ComplyRailClient with method stubs (registerVasp, submitPayment, submitAttestation, releasePayment, rejectPayment, getVaspStatus, getPaymentStatus)
- `ivms101.ts` - IVMS101Builder for constructing compliance messages
  - Builder pattern: `IVMS101Builder.create().setOriginatingVasp(...).setBeneficiary(...).buildAndHash()`
  - Support for natural and legal persons with FATF-compliant fields
  - Message validation with detailed error reporting
  - Helper functions: `createNaturalPerson()`, `createLegalPerson()`
- `crypto.ts` - SHA256 utilities for message hashing
  - `sha256(data)` - returns hex string
  - `sha256Buffer(data)` - returns Buffer
  - `sha256Bytes32(data)` - returns 0x-prefixed 32-byte hex

**Tests:** 14 passing
- `tests/crypto.test.ts` - 6 tests covering hash consistency, buffer/string inputs
- `tests/ivms101.test.ts` - 8 tests covering message building, validation, hashing, person types

**Examples:**
- `examples/below-threshold.ts` - Auto-released payment flow (< threshold)
- `examples/above-threshold.ts` - Attestation-required flow (> threshold) with IVMS101 message

**Build:**
- ESM build: `dist/index.mjs` (5.33 KB)
- CJS build: `dist/index.js` (6.78 KB)
- Type declarations: `dist/index.d.ts`
- Config: `tsup.config.ts`, `vitest.config.ts`

## Git Configuration

All repos configured with:
- User: Benjamin Johnson (benjaminjohnsonfin)
- Email: benjaminjohnsonfin@gmail.com
- Commits use "Benjamin Johnson" (no Claude co-author trailers per requirements)

### GitHub Authentication
- Authenticated via `gh auth login --with-token`
- Token stored in `~/.config/gh/hosts.yml`
- Git credential helper configured: `git config --global credential.helper gh`

## Next Steps

### Phase 4 - App Dashboard (Not started)
**Scope:**
- Wallet authentication (Stellar Wallets Kit)
- VASP directory page
- Threshold configuration UI
- Payment queue with status tracking
- Manual release/reject interface
- Audit log export (CSV/JSON)
- Role-based access control

### Phase 5 - Indexer & Audit (Not started)
- Contract event indexer → Postgres
- Audit log database schema
- CSV/JSON export
- RBAC enforcement

### Phase 6 - E2E Integration (Not started)
- Full two-VASP payment flow test
- Compliant (attestation) + non-compliant scenario
- Dashboard integration

## Important Notes

### Compliance & Legal
ComplyRail is a technical tool, not legal advice. FATF Travel Rule obligations vary by jurisdiction (EU TFR, FinCEN rules, MAS requirements) and carry real regulatory liability. Any production deployment requires legal/compliance review.

### Contract Development
- Deprecation warnings suppressed via `#![allow(deprecated)]` in `/workspaces/complyrail-contracts/src/lib.rs`
- Plan to refactor to newer soroban-sdk APIs in future phases
- Cargo dependencies use wildcard version (resolves to latest compatible)

### SDK Implementation Notes
- ComplyRailClient contract methods (registerVasp, submitPayment, etc.) are stubs returning mock transaction IDs
- Full Soroban SDK integration for contract invocation deferred to Phase 4
- Stellar SDK v11 imported; Server class and contract invocation APIs pending implementation

## Testing & Building

### Contracts
```bash
cd /workspaces/complyrail-contracts
cargo test --lib                          # Unit tests
docker run --rm -v $(pwd):/workspace complyrail-build:latest  # Docker build
```

### SDK
```bash
cd /workspaces/complyrail-sdk
npm test                                  # Run vitest
npm run build                             # Build ESM + CJS
npm run type-check                        # Type checking
```

### App
```bash
cd /workspaces/complyrail-app
npm ci && npm run dev                     # Install and run dev server
```

## Repository URLs

- **Contracts:** https://github.com/ComplyRail/complyrail-contracts
- **App:** https://github.com/ComplyRail/complyrail-app
- **SDK:** https://github.com/ComplyRail/complyrail-sdk

## Resumption Instructions

1. **Authentication:**
   ```bash
   gh auth status  # Verify token is still valid
   ```

2. **Start Phase 4 or continue Phase 3:**
   ```bash
   cd /workspaces/complyrail-app  # Phase 4
   cd /workspaces/complyrail-sdk  # Phase 3 refinements
   ```

3. **Push after each discrete feature:**
   - One feature = one commit = one push
   - Use conventional commits: `feat(sdk):`, `fix(contracts):`, etc.
   - Example: `feat(sdk): add Soroban contract invocation layer`

## Git Log (Recent)

```
contracts: 8a71fa4..6e11c83 (7 commits)
app: 2 commits (setup)
sdk: deeba8b (Phase 3 implementation, local only - needs push)
```

## Known Issues

1. **Git push authentication** - Phase 3 SDK commit is local only; needs token verification or org setup review before pushing
2. **ComplyRailClient stubs** - Contract method implementations pending Phase 4 Soroban SDK work
3. **Deprecation warnings** - Soroban SDK needs refactoring to newer APIs (deferred to Phase 4-5)

---

**Last Updated:** 2026-08-14  
**Phase:** 3 (SDK complete, local only)  
**Next Phase:** Phase 4 - App Dashboard  
**Effort Estimate:** 40-60 hours total (Phases 3-5)
