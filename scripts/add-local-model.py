#!/usr/bin/env python3
"""Add a pinned Hugging Face model bundle to the bundled isoboard catalog."""

import argparse
import json
import os
from pathlib import Path
import re
import sys
import tempfile
from urllib.parse import quote, urlparse
from urllib.request import Request, urlopen


CATALOG = Path(__file__).resolve().parents[1] / "src-tauri/src/local_models/catalog.json"


def component(value):
    return value not in ("", ".", "..") and re.fullmatch(r"[A-Za-z0-9_.-]+", value)


def repository(value):
    if "://" in value:
        url = urlparse(value)
        if url.scheme != "https" or url.netloc not in ("huggingface.co", "hf.co"):
            raise ValueError("Use an HTTPS Hugging Face repository URL or owner/repo.")
        parts = url.path.strip("/").split("/")
        if len(parts) < 2:
            raise ValueError("The repository needs an owner and name.")
        value = "/".join(parts[:2])
    parts = value.split("/")
    if len(parts) != 2 or not all(component(part) for part in parts):
        raise ValueError("The repository must be owner/repo.")
    return value


def metadata(repo, revision=None):
    endpoint = "https://huggingface.co/api/models/" + repo
    if revision:
        endpoint += "/revision/" + quote(revision, safe="")
    request = Request(endpoint + "?blobs=true", headers={"User-Agent": "isoboard-catalog-helper"})
    with urlopen(request, timeout=30) as response:
        return json.load(response)


def choose(files, selected, projector):
    candidates = [
        file for file in files
        if file["rfilename"].endswith(".gguf")
        and file["rfilename"].rsplit("/", 1)[-1].startswith("mmproj") == projector
    ]
    if selected:
        candidates = [file for file in files if file["rfilename"] == selected]
    if len(candidates) != 1:
        flag = "--projector-file" if projector else "--model-file"
        names = ", ".join(file["rfilename"] for file in candidates) or "none"
        raise ValueError(f"Choose {flag} explicitly. Candidates: {names}")
    file = candidates[0]
    name = file["rfilename"]
    lfs = file.get("lfs") or {}
    size, sha = lfs.get("size"), lfs.get("sha256")
    if not component(name) or not name.endswith(".gguf"):
        raise ValueError("Only GGUF files at the repository root are supported.")
    if type(size) is not int or size <= 0 or file.get("size") != size:
        raise ValueError(f"Missing or inconsistent byte count for {name}.")
    if not isinstance(sha, str) or not re.fullmatch(r"[0-9a-f]{64}", sha):
        raise ValueError(f"Missing SHA-256 metadata for {name}.")
    return {"name": name, "bytes": size, "sha256": sha}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("repo", help="Hugging Face URL or owner/repo")
    parser.add_argument("--id", required=True, help="Stable unique catalog ID")
    parser.add_argument("--name", required=True, help="Display name")
    parser.add_argument("--prompt", choices=("standard", "isocr"), required=True)
    capabilities = parser.add_mutually_exclusive_group(required=True)
    capabilities.add_argument("--text-capable", action="store_true")
    capabilities.add_argument("--extraction-only", action="store_true")
    parser.add_argument("--revision", help="Commit, tag or branch to resolve and pin")
    parser.add_argument("--minimum-ui-mode", choices=("basic", "advanced", "debug"), default="basic",
                        help="Lowest UI mode that shows this model in the picker (default: basic)")
    parser.add_argument("--model-file")
    parser.add_argument("--projector-file")
    parser.add_argument("--catalog", type=Path, default=CATALOG)
    parser.add_argument("--dry-run", action="store_true", help="Print the entry without writing")
    args = parser.parse_args()
    if not component(args.id) or not args.name.strip():
        raise ValueError("Supply a safe ID and a nonempty display name.")
    original = args.catalog.read_bytes()
    catalog = json.loads(original)
    if catalog.get("version") != 1 or not isinstance(catalog.get("models"), list):
        raise ValueError("Expected a version-one model catalog.")
    if any(model["id"] == args.id for model in catalog["models"]):
        raise ValueError(f"Catalog ID already exists: {args.id}. Existing entries are not overwritten.")
    repo = repository(args.repo)
    revision = metadata(repo, args.revision).get("sha")
    if not isinstance(revision, str) or not re.fullmatch(r"[0-9a-f]{40}", revision):
        raise ValueError("Hugging Face did not return a commit SHA.")
    # Resolve the file metadata at the pinned commit even if the branch advances.
    pinned = metadata(repo, revision)
    if pinned.get("sha") != revision:
        raise ValueError("Repository metadata did not match the pinned revision.")
    files = pinned.get("siblings", [])
    model = choose(files, args.model_file, False)
    projector = choose(files, args.projector_file, True)
    if model["name"] == projector["name"]:
        raise ValueError("The model and projector must be different files.")
    if model["bytes"] + projector["bytes"] > 2**64 - 1:
        raise ValueError("Bundle byte count is too large.")
    entry = {
        "id": args.id,
        "name": args.name.strip(),
        "supports_text": args.text_capable,
        "minimum_ui_mode": args.minimum_ui_mode,
        "prompt": args.prompt,
        "download": {"repo": repo, "revision": revision, "files": [model, projector]},
    }
    if args.dry_run:
        print(json.dumps(entry, indent=2))
        return
    catalog["models"].append(entry)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", dir=args.catalog.parent, delete=False) as output:
            temporary = Path(output.name)
            json.dump(catalog, output, indent=2)
            output.write("\n")
            output.flush()
            os.fsync(output.fileno())
        if args.catalog.read_bytes() != original:
            raise ValueError("Catalog changed while writing; retry the command.")
        os.chmod(temporary, args.catalog.stat().st_mode & 0o777)
        os.replace(temporary, args.catalog)
    finally:
        if temporary and temporary.exists():
            temporary.unlink()
    print(f"Added {args.name.strip()}: {model['bytes'] + projector['bytes']:,} bytes at {revision}")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(f"Error: {error}", file=sys.stderr)
        sys.exit(1)
