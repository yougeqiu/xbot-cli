# Activity lifecycle implementation plan

The second-batch request is the specification. Work on `feature/activity-lifecycle`, based on `62d46dd`. The legacy tree at `E:/zhongzhuan/gitlab-shadow-rpa/xbot-cli` is read-only; real-client acceptance belongs to the user.

- [x] Trace legacy activity commands and installation evidence. The legacy 0.4.2 fix is already present in the embedded source: download completion precedes dependency notifications, dirty marking, saves and Python task completion. Preserve this ordering and add an executable regression.
- [x] Add failing MockInproc contracts for creation, recovery, consumer creation, install/upgrade, metadata, offline refusal, invalid inputs, mismatches and unknown writes.
- [x] Add clap activity commands and prepared operations with offline previews, local validation and bridge-only execution. Verify current identity and persisted results; never retry a write automatically.
- [x] Expose metadata maintenance through an Origin-protected bridge route, preserving closed-Studio checks, the legacy path allowlist, immutable package identity, full prevalidation, backups and disk readback.
- [x] Rebuild committed payload artifacts, document app sync/publish equivalents and automatic metadata backup, and deliver a PR description plus real-client acceptance checklist.
- [x] Run fmt, clippy with warnings denied, all Rust tests and isolated payload tests; review and commit the completed batch. Rust: 190 passed, 2 existing ignored. Node: 36 passed. Skill: 16 passed. Static release build, offline package installation and packaged activity smoke checks passed. Real-client acceptance remains with the user.

Scope decisions: retain the legacy flag names. Expose `activity current`, `probe` and `install-state` as read-only evidence. Existing `app sync`/`app publish` remain the equivalent commands; do not add duplicate lifecycle implementations or the client-api catalog. Every new activity operation uses the same inproc bridge. Metadata maintenance accepts a complete manifest and never performs workspace reload or ordinary CodeFlow source writes.
