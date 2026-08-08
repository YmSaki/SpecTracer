# Fixture Plan

The `calc` fixture is intentionally small and deterministic. It contains a
normal registered test, table-driven cases, an unregistered test, and known
static-audit failures. The `.verify/` records are canonical inputs for scanner
and later record-management acceptance tests; generated indexes and logs must
remain outside versioned fixture data.

