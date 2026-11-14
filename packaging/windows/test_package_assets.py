"""Cross-platform input checks: python3 packaging/windows/test_package_assets.py.

No Windows tools or downloads needed. The MSI and portable package must only
reference existing, attributed artwork, not optional local branding files.
"""
import hashlib
from pathlib import Path
import re
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


class PackageAssets(unittest.TestCase):
    def test_all_referenced_icon_inputs_exist(self):
        script = (ROOT / "packaging/windows/package.ps1").read_text()
        paths = re.findall(r"'((?:assets\\app-icon\\)[^']+)'", script)
        self.assertTrue(paths, "package script must reference its app artwork")
        for path in paths:
            with self.subTest(path=path):
                asset = ROOT / path.replace("\\", "/")
                self.assertTrue(asset.is_file(), f"missing package input: {asset}")
                self.assertGreater(asset.stat().st_size, 0)

    def test_portable_validator_accepts_packaged_executable_pair(self):
        package = (ROOT / "packaging/windows/package.ps1").read_text()
        validator = (ROOT / "packaging/windows/test-portable.ps1").read_text()
        # The unconditional staging copy is the minimum payload, even without aliases.
        staged = re.search(r"^Copy-Item (.+) \$Stage$", package, re.MULTILINE)
        self.assertIsNotNone(staged)
        executables = set(re.findall(r"'([^']+\.exe)'", staged[1]))
        self.assertEqual(len(executables), 2)
        accepted = re.search(r"\$ExecutableStems = @\(([^)]+)\)", validator)
        self.assertIsNotNone(accepted)
        stems = re.findall(r"'([^']+)'", accepted[1])
        self.assertTrue(any({f"{stem}.exe", f"{stem}-cli.exe"} == executables for stem in stems))
        self.assertTrue({"pdfale", "alepdf"}.issubset(stems), "keep legacy preview pairs")
        self.assertIn('Join-Path $Root "$Stem.exe") -PathType Leaf) -and', validator)
        self.assertIn('Join-Path $Root "$Stem-cli.exe") -PathType Leaf)', validator)
        self.assertIn("if (-not $ExeName) { throw", validator)

    def test_msi_icon_matches_attributed_artwork(self):
        script = (ROOT / "packaging/windows/package.ps1").read_text()
        match = re.search(r'-d "IconPath=\$\(Join-Path \$Root \'([^\']+)\'\)"', script)
        self.assertIsNotNone(match, "WiX must receive the repository icon path")
        path = match[1].replace("\\", "/")
        manifest = tomllib.loads((ROOT / "ATTRIBUTION.toml").read_text())
        entry = next((asset for asset in manifest["asset"] if asset.get("path") == path), None)
        self.assertIsNotNone(entry, f"unattributed installer icon: {path}")
        self.assertEqual(hashlib.sha256((ROOT / path).read_bytes()).hexdigest(), entry["sha256"])
        self.assertEqual(entry["licence_file"], "assets/app-icon/LICENSE.txt")
        self.assertIn("'assets\\app-icon\\LICENSE.txt'", script)
        self.assertIn("'LICENSE-app-icon.txt'", script)


if __name__ == "__main__":
    unittest.main()
