# AudioRouter contracts

This package is the checked-in TypeScript view of the shared JSON-RPC and
domain contracts. It is intentionally transport-only: it does not open audio
devices or grant permissions. Run `npm install` once, then run:

```powershell
npm run typecheck
npm run check:drift
```

On Windows hosts where PowerShell blocks `npm.ps1`, use `npm.cmd` for the same
commands. The drift check compares the TypeScript surface with the
authoritative CLI method/node/processor catalogs.
