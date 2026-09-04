"""Check tracked publication inputs without printing any matched secret values."""
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
FORBIDDEN = {".apk", ".apks", ".apkm", ".xapk", ".aab", ".aar", ".jar", ".so",
             ".dll", ".exe", ".p12", ".jks", ".keystore", ".pem", ".key", ".idsig"}
PATTERNS = [
    rb"gh[pousr]_[A-Za-z0-9]{30,}",
    rb"github_pat_[A-Za-z0-9_]{30,}",
    rb"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----",
    rb"https://(?:canary\.|ptb\.)?discord(?:app)?\.com/api/webhooks/[0-9]+/[A-Za-z0-9_-]+",
    rb"AKIA[0-9A-Z]{16}",
]


def check():
    paths = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT).decode().split("\0")
    errors = []
    for name in filter(None, paths):
        path = ROOT / name
        if not path.is_file():
            errors.append(f"Missing tracked file: {name}")
            continue
        if (path.suffix.lower() in FORBIDDEN or path.name == "signing.json"
                or path.name == ".env" or path.name.startswith(".env.") and path.name != ".env.example"
                or name.startswith((".ympatch/", "dist/", "target/", "reports/android/", "compatibility/android/reports/"))):
            errors.append(f"Private/generated artifact: {name}")
        content = path.read_bytes()
        if len(content) > 4 * 1024 * 1024:
            errors.append(f"Oversized source artifact: {name}")
        if any(re.search(pattern, content) for pattern in PATTERNS):
            errors.append(f"Potential credential: {name}")
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print("Publication check passed: no prohibited artifacts or known credential patterns.")
    return 0


if __name__ == "__main__":
    sys.exit(check())
