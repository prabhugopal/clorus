# Parity Checklist

Canonical parity status now lives in:

- `docs/generated/PARITY_STATUS.md`
- `docs/generated/STATUS.json`

This file stays as a stable pointer so older links do not break.

## Update Command

```bash
python3 scripts/docs/build_status.py
```

## Notes

- The generated status pages are derived from a curated inventory of source and
  test references.
- Historical parity notes remain elsewhere in `docs/`, but status claims should
  resolve to the generated pages above.
