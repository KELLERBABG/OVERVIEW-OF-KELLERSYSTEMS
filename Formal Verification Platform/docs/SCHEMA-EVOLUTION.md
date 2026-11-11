# Keller-Verify — Schema Evolution & Migration Policy (P2.9)

This document establishes the schema evolution and backward compatibility guarantees
for Keller-Verify machine-readable outputs (`report.json`, `report.run.json`, and
`report.environment.json`).

The formal JSON Schema definitions are versioned and embedded directly in the binary:
- `schema/report.schema.json` (accessible via `keller schema report`)
- `schema/run.schema.json` (accessible via `keller schema run`)

---

## 1. Schema Versioning Principles

All generated run and environment files carry explicit schema version metadata:
```json
{
  "$schema": "https://kellersystems.dev/schemas/run-v1.json",
  "schema_version": 1,
  ...
}
```

The integer `schema_version` guarantees structural compatibility for automated tools,
monitoring dashboards, and CI quality gates.

---

## 2. Compatibility Guarantees

### Non-Breaking Changes (No version bump required)
The following modifications maintain backward compatibility and may occur within the
same `schema_version`:
1. **Adding optional fields**: New diagnostic metadata (e.g., timestamps, memory stats)
   may be introduced. Consumers must ignore unknown fields without failing.
2. **Expanding taxonomy variants**: New `kind` values (e.g., newly modeled assertion
   types) or new `defect_class` strings can be added. Consumers should treat unknown
   classes as unclassified.
3. **Enhancing details formatting**: Diagnostic string details may provide additional
   context or witness expressions without altering structured fields.

### Breaking Changes (Mandatory `schema_version` bump)
Any modification that could break existing consumers requires incrementing `schema_version`:
1. Removing, renaming, or changing the data type of an existing field.
2. Changing an optional property to required or vice versa.
3. Restricting or removing valid enum values in existing required fields.
4. Restructuring root arrays into objects or changing array nesting levels.

---

## 3. Consumer Migration Guide

### Validating Against the Pinned Schema
Consumers and CI pipelines can dump the active formal schema and validate artifacts:
```bash
# Dump the formal schema
keller schema report > report.schema.json

# Validate report.json against schema using jsonschema-cli or ajv
npx ajv-cli validate -s report.schema.json -d report.json
```

### Upgrading to New Schema Versions
When upgrading Keller-Verify across a major version bump that increments `schema_version`:
1. Check the `schema_version` integer property before parsing payload fields.
2. If consuming older stored reports (e.g., historical baselines), implement version-based
   field decoders:
   ```python
   def parse_keller_report(data):
       version = data.get("schema_version", 1)
       if version == 1:
           return parse_v1_report(data)
       elif version == 2:
           return parse_v2_report(data)
       else:
           raise ValueError(f"Unsupported schema version: {version}")
   ```
3. Use the baseline migration helper `python scripts/progress-metric.py --write-baseline`
   to capture updated baseline assertions when schema fields evolve.
