"""Validate channel/version metadata used by CI and release workflows."""
import argparse
import json
import pathlib
import subprocess
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag")
    parser.add_argument("--check-branch", action="store_true")
    args = parser.parse_args()
    config = json.loads((ROOT / "release.json").read_text())
    package = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]
    lane, version = config["lane"], config["version"]
    if lane not in ("stable", "dev") or config["upstreamChannel"] not in ("stable", "beta"):
        raise SystemExit("Invalid release lane or upstream channel")
    if version != package["version"]:
        raise SystemExit("release.json and Cargo.toml versions differ")
    locked = tomllib.loads((ROOT / "Cargo.lock").read_text())
    if not any(p["name"] == "ympatcher" and p["version"] == version for p in locked["package"]):
        raise SystemExit("Cargo.lock version differs")
    if args.tag and args.tag != "v" + version:
        raise SystemExit("Tag must match release.json version")
    if args.check_branch:
        subprocess.run(["git", "fetch", "origin", f"{lane}:refs/remotes/origin/{lane}"], cwd=ROOT, check=True)
        subprocess.run(["git", "merge-base", "--is-ancestor", "HEAD", f"origin/{lane}"], cwd=ROOT, check=True)
    print(f"{lane}: v{version}, upstream {config['upstreamChannel']}")


if __name__ == "__main__":
    main()
