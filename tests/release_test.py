"""Run with python3 tests/release_test.py (the release job runs on Linux)."""
import os
from pathlib import Path
import runpy
import tempfile
from zipfile import ZipFile

root = Path(__file__).resolve().parents[1]
prepare = runpy.run_path(str(root / "scripts/prepare-release.py"))["prepare"]
original = Path.cwd()
with tempfile.TemporaryDirectory(prefix="langu-release-check-") as folder:
    try:
        os.chdir(folder)
        Path("src-tauri").mkdir()
        for name in ("package.json", "src-tauri/tauri.conf.json"):
            Path(name).write_bytes((root / name).read_bytes())
        Path("installers").mkdir()
        names = [
            "兰谷多账号工具_0.1.0_x64.dmg",
            "兰谷多账号工具_0.1.0_aarch64.dmg",
            "兰谷多账号工具_0.1.0_x64-setup.exe",
            "兰谷多账号工具_0.1.0_x64_zh-CN.msi",
            "langu-account-tool_0.1.0_amd64.deb",
            "langu-account-tool-0.1.0-1.aarch64.rpm",
            "兰谷多账号工具_0.1.0_amd64.AppImage",
        ]
        for name in names:
            Path("installers", name).write_bytes(b"installer test bytes")
        prepare()
        assets = Path("release-assets.txt").read_text(encoding="utf-8").splitlines()
        assert len(assets) == len(names)
        for asset in assets:
            archive, label = asset.split("#")
            assert Path(archive).name.isascii()
            assert not Path(archive).name.startswith("langu-account-tool")
            with ZipFile(archive) as bundle:
                assert len(bundle.infolist()) == 1
                item = bundle.infolist()[0]
                assert item.filename.startswith("兰谷多账号工具")
                assert item.flag_bits & 0x800  # Explicit UTF-8 entry names.
                assert bundle.read(item) == b"installer test bytes"
                assert label == item.filename + ".zip"
        print("Release ZIP names, Chinese labels and installer contents passed")
    finally:
        os.chdir(original)
