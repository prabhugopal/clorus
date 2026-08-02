# Coverage

Canonical status lives in:

- `docs/generated/PARITY_STATUS.md`  - human canonical source
- `docs/generated/STATUS.json`       - machine canonical source

`docs/generated/COVERAGE_SUMMARY.md` is a derived coverage view generated from the
same source inventory.

This file remains as the stable entrypoint for older references.

## Update Command

```bash
python3 scripts/docs/build_status.py
```

## Scope

- Test inventory counts
- Feature status by area
- Generated canonical coverage summary for MkDocs
