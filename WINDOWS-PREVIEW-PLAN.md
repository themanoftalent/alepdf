# Windows preview validation — prepared, not built

This isolated checkout inherits the already tested AlePdf branding and safe-save changes from the macOS review branch. The installed Mac app and original source checkout are unchanged.

`packaging/windows/package.ps1 -Arch x64 -PortableOnly` reuses existing static-CRT build, PE architecture/subsystem checks, legal notices, verified OCR models, packaging and CLI version smoke test. The added switch skips MSI creation, avoiding WiX and installer/registry changes. Without the switch the prior MSI path remains.

`.github/workflows/windows-preview.yml` provides a Windows-only hosted runner: Rust 1.95.0, Windows Drive collision tests, unsigned portable build, SHA-256 and signature status, and the distribution asset gate. It deliberately uploads no binaries and creates no release. Logs are public in the public repository. It triggers only on `feat/windows-preview` or explicit dispatch. No signing secrets, permissions beyond contents:read, paid service or macOS job are requested.

## Required next remote operation
Approve creating/pushing branch `feat/windows-preview` in `themanoftalent/alepdf`, including inherited local fixes, PortableOnly and this workflow. This push triggers the Windows-only validation run. Do not push main or tag a release. GitHub connector has repository push/admin access; local gh credential currently reports invalid. Remote write/CI execution has NOT occurred.

## Release holds
- This Mac has only Apple targets installed; no Windows VM, Wine, PowerShell, or native Windows runtime was found. No EXE has been built or run in this phase.
- The original 14 asset findings are resolved in this Windows checkout. Licensed original SVGs were rendered back into their PNG/ICO derivatives; ICNS retains a regenerated 1024px PNG for source consistency. The unverified replacement artwork is gone, not re-attributed. The three unattributed assets were removed, along with the README reference. Restricted ArtCraft logo files, unused embedding helpers, and the one screenshot explicitly documenting an embedded ArtCraft mark were removed. Original NOTICE and trademark-license text remain.
- `xtask assets --write` and `xtask assets` pass: 280 repository assets, 6 bundled dependencies, 22 fetched inputs. This is a repository policy check, not certification of the complete Windows executable/dependency license inventory. The future archive must still be inspected.
- Portable ZIP now carries the original app-icon licence plus all three bundled Latin-font OFL notices. `test-portable.ps1` will extract the real ZIP, verify required files/notices and x64 PE headers, and run the extracted CLI. It does not claim GUI acceptance.
- Existing MSI checks expect Learning Machines LLC while MSI declares SoftAI; installer identities/registry policy require a separate deliberate reconciliation before shipping MSI. Portable validation avoids claiming that MSI is ready.
- Runtime acceptance must use actual Windows: launch GUI, open a synthetic PDF, annotate/save/reopen, Fill & Sign, collision Cancel/Keep both/Replace, repeated saves, and clean portable extraction. A compiled EXE or --version result is not this acceptance.

The public website remains unchanged and offers no fictitious Windows download. After gates pass, publish a clearly labeled unsigned preview package/checksum to an authorized release destination, then add and anonymously verify the actual download on Vercel. Do not include security-warning bypass instructions.

## Local verification (2026-10-31)

- Built xtask offline using an isolated target directory; build passed.
- Asset manifest regeneration and audit passed; dependency layering passed (37 crates).
- Workflow YAML syntax and diff whitespace checks passed. Touched widgets file formatted.
- Restored 256px original icon visually inspected; matches the licensed lion source.
- Windows package/test scripts remain unexecuted locally because PowerShell and Windows are unavailable.
- No remote push, workflow run, binary upload, website modification, or Mac app change occurred.

## Requested EXE installer

Added `preview-setup.iss` and a workflow compile step using the Windows runner's preinstalled Inno Setup. Per-user, x64-only preview installer, separate AlePdfPreview folder/identity; no elevation, file associations, services, scheduled tasks or automatic app launch. It bundles the validated portable directory including notices. Compilation, installation/uninstallation, and GUI acceptance have NOT run. No EXE exists yet. The existing MSI path is not presented as tested.

## Authorization update

The owner approved branch push, Windows CI, separate Mac release build, and publication of verified preview installers. The workflow now validates EXE installation/uninstallation and GUI window startup, uploads validated artifacts, and publishes an explicitly unsigned Windows prerelease only after all preceding checks pass. Interactive PDF acceptance is not represented as completed. Earlier pending-approval notes above are historical.
