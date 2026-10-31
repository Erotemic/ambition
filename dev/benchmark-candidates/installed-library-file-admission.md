# Installed library files need the installed path

An embedded library index can name image bytes that remain external to the
executable. A filesystem check that accepts only native source paths or
embedded bytes rejects those images even when the package has every file.
The small embedded fallback set still draws, which can look like partial
asset deployment.

The catalog and the library reader must use the same package path:
`assets/__libraries/<library-id>/<logical-path>` under the installed asset root.
Keep missing optional files optional. Do not use a global freshness requirement
to compensate for an incorrect runtime path.

Test an installed tree outside the source checkout. Check that the catalog
admits its external images, the reader returns their bytes, and absent optional
images or quality tiers use the existing fallback. A package hash alone cannot
prove runtime admission. The advisory content-status query answers freshness;
it does not replace either check.
