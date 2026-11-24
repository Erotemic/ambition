# Installed library files need the installed path

An embedded library index can name image bytes that remain external to the
executable. A filesystem check that accepts only native source paths or
embedded bytes rejects those images even when the package has every file.
The small embedded fallback set still draws, which can look like partial
asset deployment.

The catalog and the library reader must use the same package path:
`assets/__libraries/<library-id>/<package-path>` under the installed asset root.
A published file's package path is its immutable publication address below the
library. Its logical name is an alias, and can differ from that address. Keep
both names in the embedded index. Export the compiler's selected file inventory
to the packager; do not select the publication again in another reader.
Resolve an unqualified library image alias before the platform checks for its
bytes. A loader cannot correct the address if the catalog rejects it first.
Keep missing optional files optional. Do not use a global freshness requirement
to compensate for an incorrect runtime path.

Test an installed tree outside the source checkout. Check that the catalog
admits its external images, the reader returns their bytes, and absent optional
images or quality tiers use the existing fallback. Install only publication
paths in the fixture. Read each file through both its logical and publication
addresses. A package hash alone cannot
prove runtime admission. The advisory content-status query answers freshness;
it does not replace either check.
