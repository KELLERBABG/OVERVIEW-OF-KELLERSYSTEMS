# Product Model Decision — Single-User Local Vault

**Status:** accepted **2026-11-11** (E9).
**Decision owner:** project owner.
**Applies to:** `fhe-server`, `fhe-client`, `apps/fhe-phone`, and every
deployment described by this repository.

---

## 1. Decision

FHE-DB is a **single-user, local-first encrypted vault**, not a multi-tenant
service.

One trusted operator runs the server on a machine they control, one holder of
the `ClientKey` uses it, and the server binds to loopback. The phone shell is
the reference product: an embedded server plus a real client SDK on the same
device. "Authentication" in this build is an **operator access control** for a
local process (optional bearer token), not user identity, and there is no
authorization or tenancy layer.

This decision is recorded here because audit finding **F9** (no
authorization/tenancy; shared token; permissive CORS) cannot be "fixed" by
adding a token check and calling it auth. It can only be accepted for a defined
product scope, or treated as a release blocker. The project chooses the
former, with the conditions below.

## 2. What the product promises

For the stated scope:

1. The server never receives plaintext **values**; it stores ciphertexts and
   public metadata (schema, row counts, timing, sizes).
2. Only the holder of the `ClientKey` can decrypt.
3. A crash never loses a mutation the server acknowledged (WAL durability).
4. The local key store is encrypted at rest under a PIN, and PIN change,
   backup import, and destruction are atomic.

It does **not** promise isolation between local OS users or processes, remote
multi-user access, per-table authorization, or protection against a malicious
server returning wrong results.

## 3. Deployment conditions

F9 is accepted **only** while all of these hold:

- **Loopback only.** The server binds `127.0.0.1` / `::1`. It is not exposed to
  a LAN, WAN, container network, reverse proxy, or browser origin.
- **One OS user.** The machine account is the trust boundary. Another local
  process or user with access to the port, the data directory, or the phone
  key store is outside the threat model (see `THREAT_MODEL.md` §6).
- **One key holder.** No sharing of the `ClientKey`, the PIN, or an exported
  backup between people.
- **No untrusted web origin.** `app()` enables permissive CORS; do not point a
  browser page you do not control at the API.
- **No managed/hosted offering.** No control plane, tenant provisioning, or
  shared database serves more than one principal.

If any condition is violated, **F9 reopens as a release blocker**; it is not
silently inherited by the new deployment.

## 4. Why this is defensible now

- The shipped product surface is exactly the local case: `fhe-phone` embeds the
  server and client in one process and talks only to `127.0.0.1`.
- The CLI defaults to loopback, and `SECURITY.md` documents that loopback is
  not a user boundary.
- The bearer token is honest about what it is: one operator secret with
  constant-time comparison, not an identity system. It is optional because on
  a single-user local machine the OS account already gates access.
- Every document that describes the server now states the model rather than
  implying multi-user safety.

## 5. What would have to change for multi-tenant or remote use

This is deliberately **not** implemented; it is the requirements list if the
decision is revisited:

1. Real authentication with per-user identity (OIDC or equivalent), not a
   shared token.
2. Per-user key envelopes and tenant-scoped storage; no cross-tenant reads by
   construction, not by convention.
3. Authorization checks on every route (table/column ownership, model
   ownership, key upload), deny-by-default.
4. Per-principal rate limits and quotas; no reliance on peer IP behind a
   proxy.
5. Audit logging of principals and administrative actions.
6. TLS on by default, token issuance/rotation, replay protection, and a
   restrictive CORS policy.
7. A revised threat model that treats the server operator and other tenants as
   adversaries, plus an external review of that model.

Until then, a deployment that needs any of the above is out of scope and must
not be described as supported.

## 6. Relationship to the audit package

- F9 is recorded as **Accepted (single-user local vault)**, not fixed, in
  [`README.md`](README.md). The acceptance is bounded by §3.
- F4/F8 (phone key lifecycle) are **fixed** and are not part of this
  acceptance; their residual risks (offline PIN guessing, TFHE-rs key types
  not exposing zeroization) are documented in
  [`KEY_LIFECYCLE.md`](KEY_LIFECYCLE.md).
- F3, F5, F6, F7, and F10 remain open and are unaffected by this decision.
- The external cryptographic audit is still required. This decision is about
  product scope, not about the security of the FHE construction.

## 7. Consequences for operations

- Operator runbooks must say "local machine" explicitly, including the WAL
  recovery guide.
- Packaging (stage 4) must not add a remote-listening default or a hosted
  control plane under this model.
- Marketing and README language may claim "local-first encrypted vault"; it
  must not claim "multi-user", "tenant isolation", "secure remote access", or
  "authorization".
