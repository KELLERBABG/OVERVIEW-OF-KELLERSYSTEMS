# ADR-001: Use TypeScript (Strict Mode)

## Status
Accepted

## Context
The project requires type safety across a distributed system with many interacting packages. JavaScript alone would lead to runtime errors in P2P messaging, cryptographic operations, and task serialization.

## Decision
Use TypeScript with strict mode enabled (`strict: true`, `noUncheckedIndexedAccess: true`, `noImplicitReturns: true`). Use `tsup` for bundling libraries and `Vite` for browser apps.

## Consequences
- Catches type errors at compile time rather than runtime
- Better IDE support and refactoring capabilities
- Slight increase in build complexity but worth the safety