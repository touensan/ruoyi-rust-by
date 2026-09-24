import hashlib
import importlib.util
import json
import subprocess
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / 'smooth-release.py'
SPEC = importlib.util.spec_from_file_location('smooth_release', SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ReleaseIntegrityTest(unittest.TestCase):
    def test_prepare_checks_archive_and_links_shared_uploads(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            package = root / 'package'
            package.mkdir()
            binary = package / 'app'
            binary.write_bytes(b'\x7fELF\x02' + b'\x00' * 13 + (62).to_bytes(2, 'little'))
            asset = package / 'web/admin/assets/app.123.js'
            asset.parent.mkdir(parents=True)
            asset.write_bytes(b'frontend')
            (package / 'runtime/uploads').mkdir(parents=True)
            (package / 'FILE_SHA256SUMS').write_text(''.join(
                '%s  ./%s\n' % (hashlib.sha256(p.read_bytes()).hexdigest(), p.relative_to(package))
                for p in (binary, asset)
            ))
            archive = root / 'release.tar.gz'
            with tarfile.open(archive, 'w:gz') as tar:
                tar.add(package, arcname='.')
            uploads = root / 'uploads'
            uploads.mkdir()
            cfg = {
                'state_dir': str(root / 'state'), 'releases_dir': str(root / 'releases'),
                'assets_dir': str(root / 'assets'), 'shared_uploads': str(uploads),
                'binary': 'app', 'web_assets': 'web/admin/assets',
                'upload_path': 'runtime/uploads',
            }
            config = root / 'config.json'
            config.write_text(json.dumps(cfg))
            subprocess.run([sys.executable, str(SCRIPT), '--config', str(config), 'prepare',
                            'v1', str(archive), hashlib.sha256(archive.read_bytes()).hexdigest(), '1.0'], check=True)
            release = root / 'releases/v1'
            self.assertEqual((release / 'runtime/uploads').resolve(), uploads)
            self.assertEqual((root / 'assets/app.123.js').read_bytes(), b'frontend')

    def test_internal_checksum_detects_missing_and_changed_files(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            binary = root / 'app'
            binary.write_bytes(b'\x7fELF\x02' + b'\x00' * 13 + (62).to_bytes(2, 'little'))
            asset = root / 'public/admin/assets/app.123.js'
            asset.parent.mkdir(parents=True)
            asset.write_bytes(b'old')
            entries = [binary, asset]
            (root / 'FILE_SHA256SUMS').write_text(''.join(
                '%s  ./%s\n' % (hashlib.sha256(p.read_bytes()).hexdigest(), p.relative_to(root))
                for p in entries
            ))
            MODULE.verify_files(root, 'app')
            asset.write_bytes(b'changed')
            with self.assertRaisesRegex(RuntimeError, 'checksum mismatch'):
                MODULE.verify_files(root, 'app')

    def test_old_asset_cannot_be_overwritten(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            shared = root / 'shared'
            old = root / 'old'
            new = root / 'new'
            for release, content in ((old, b'old'), (new, b'new')):
                path = release / 'assets/app.js'
                path.parent.mkdir(parents=True)
                path.write_bytes(content)
            cfg = {'assets_dir': str(shared), 'web_assets': 'assets'}
            MODULE.assets(cfg, old)
            with self.assertRaisesRegex(RuntimeError, 'changed content'):
                MODULE.assets(cfg, new)


if __name__ == '__main__':
    unittest.main()
