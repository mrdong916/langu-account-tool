"""Keep Chinese installer names inside UTF-8 ZIPs; GitHub strips non-ASCII asset names."""
import json
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile


def prepare(source=Path("installers"), destination=Path("release")):
    product = json.loads(Path("src-tauri/tauri.conf.json").read_text())["productName"]
    package = json.loads(Path("package.json").read_text())["name"]
    destination.mkdir()
    assets = []
    for installer in sorted(source.rglob("*")):
        if not installer.is_file():
            continue
        prefix = next((name for name in (product, package) if installer.name.startswith(name)), None)
        if prefix is None or installer.suffix not in (".exe", ".msi", ".dmg", ".deb", ".rpm", ".AppImage"):
            raise ValueError(f"Unexpected installer: {installer.name}")
        suffix = installer.name[len(prefix):]
        archive = destination / (suffix.lstrip("_-") + ".zip")
        if archive.exists():
            raise ValueError(f"Duplicate installer: {archive.name}")
        chinese_name = product + suffix
        with ZipFile(archive, "w", compression=ZIP_DEFLATED) as bundle:
            bundle.write(installer, arcname=chinese_name)
        assets.append(f"{archive}#{chinese_name}.zip")
    if not assets:
        raise ValueError("No installers found")
    Path("release-assets.txt").write_text("\n".join(assets) + "\n", encoding="utf-8")


if __name__ == "__main__":
    prepare()
