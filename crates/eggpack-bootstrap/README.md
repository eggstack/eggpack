# eggpack-bootstrap

Generates exact-release POSIX and PowerShell first-install scripts from a matching DistributionContract and ReleaseManifest. Generation is pure. Scripts verify size and SHA-256 before creating a missing destination. They do not select releases, overwrite existing files, elevate privileges, or update installed applications. SHA-256 is integrity evidence, not authenticity.

M002 adds transactional bundle/archive first-install safety behind `BootstrapInstallPolicyV1`:

- direct releases keep M001 behavior (single executable, no policy required);
- bundle releases download and verify every member before any placement, then place all files atomically with invocation-only rollback;
- archive releases support tar+gzip only, verify archive bytes, check exact member inventory before extraction, validate extracted regular files by size and SHA-256, then place flattened install names atomically;
- caller-owned `Executable` (POSIX 0755) versus `Data` (POSIX 0644) modes are explicit policy and never guessed; PowerShell preserves bytes with inherited ACL;
- existing destinations are never replaced, pre-existing files are never removed, and partial placement rolls back only invocation-created paths;
- normal updates, Eggup receipts, service lifecycle, signing, and remote discovery remain out of scope.

M002a qualifies the PowerShell TarGzip path at runtime on the Windows lane (pwsh 7 plus tar.exe, both verified before the focused test): exact installed bytes, nested-source flattening, no-overwrite, pre-existing preservation, wrong archive size/SHA rejection before extraction, inner member defenses (missing/extra/traversal/absolute/backslash/symlink/directory members rejected after valid outer archive verification), member size/SHA evidence checks, missing-tar.exe bounded failure, and archive placement-conflict rollback of invocation-created files. POSIX malicious-member tests use matching outer archive digests so each case proves the intended inventory/path/type guard is reached.
