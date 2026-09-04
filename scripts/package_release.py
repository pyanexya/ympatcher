"""Package only the built CLI, with a portable SHA-256 checksum filename."""
import hashlib
import json
import pathlib
import platform
import shutil
import subprocess

root = pathlib.Path(__file__).resolve().parents[1]
config = json.loads((root / "release.json").read_text())
system = platform.system().lower()
if system not in ("windows", "linux"):
    raise SystemExit("Release packaging supports Linux and Windows")
suffix = ".exe" if system == "windows" else ""
source = root / "target" / "release" / ("ympatcher" + suffix)
version = subprocess.check_output([str(source), "--version"], text=True).strip()
if version != "ympatcher " + config["version"]:
    raise SystemExit("Built binary version differs from release.json")
output = root / "dist" / f"ympatcher-v{config['version']}-{config['lane']}-{system}-x86_64{suffix}"
output.parent.mkdir(exist_ok=True)
shutil.copy2(source, output)
digest = hashlib.sha256(output.read_bytes()).hexdigest()
output.with_name(output.name + ".sha256").write_text(f"{digest}  {output.name}\n", encoding="ascii")
print(output.name)
