# eggpack-bootstrap

Generates exact-release POSIX and PowerShell first-install scripts from a matching DistributionContract and ReleaseManifest. Generation is pure. Scripts verify size and SHA-256 before creating a missing destination. They do not select releases, overwrite existing files, elevate privileges, or update installed applications. SHA-256 is integrity evidence, not authenticity.
