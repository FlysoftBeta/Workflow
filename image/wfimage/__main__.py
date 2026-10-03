"""wfimage command line. Pure Python standard library plus the `zstd` executable.

    python3 -m wfimage tree-from-tar TAR OUTDIR
    python3 -m wfimage compare LEFT RIGHT [--prune-file F] [--ignore PATH]...
    python3 -m wfimage pack --rootfs DIR --manifest FILE --out IMAGE --arch A --profile P ...
    python3 -m wfimage verify IMAGE [--index JSON] [--arch A] [--profile P]
    python3 -m wfimage install IMAGE DEST --index JSON [--arch A]
    python3 -m wfimage fixture OUTDIR
    python3 -m wfimage to-tar IMAGE OUT.tar [--index JSON] [--devices]
    python3 -m wfimage metadata --versions ENV --arch A --base-out F [--state S --provision P --builder B --extra-out F]
"""
import argparse
import json
import sys

from . import fixture, manifest, meta, pack, tree, verify
from .common import ImageError


def _load_manifest(path):
    with open(path, "rb") as handle:
        return manifest.parse(handle.read())


def _load_json(path):
    if path is None:
        return None
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def main(argv=None):
    parser = argparse.ArgumentParser(prog="wfimage")
    commands = parser.add_subparsers(dest="command", required=True)

    command = commands.add_parser("tree-from-tar", help="rootless extraction into rootfs/ + manifest")
    command.add_argument("tar")
    command.add_argument("out")

    command = commands.add_parser("compare", help="compare two ownership manifests")
    command.add_argument("left")
    command.add_argument("right")
    command.add_argument("--prune-file")
    command.add_argument("--canonical-file")
    command.add_argument("--ignore", action="append", default=[])

    command = commands.add_parser("pack", help="rootfs directory + manifest -> image")
    command.add_argument("--rootfs", required=True)
    command.add_argument("--manifest", required=True)
    command.add_argument("--out", required=True)
    command.add_argument("--arch", required=True, choices=["arm64", "amd64"])
    command.add_argument("--profile", required=True, choices=["workspace", "base"])
    command.add_argument("--type", default="debian-trixie")
    command.add_argument("--type-version", type=int, default=1)
    command.add_argument("--base-json", required=True, help="JSON object for metadata.base")
    command.add_argument("--extra-json", help="JSON object merged into metadata (workspace fields)")
    command.add_argument("--prune-file")
    command.add_argument("--canonical-file")
    command.add_argument("--override", action="append", default=[], metavar="GUEST=HOSTFILE")
    command.add_argument("--no-compress", action="store_true")
    command.add_argument("--level", type=int, default=19)
    command.add_argument("--window-log", type=int, default=27)

    command = commands.add_parser("verify", help="validate an image against the contract")
    command.add_argument("image")
    command.add_argument("--index")
    command.add_argument("--arch")
    command.add_argument("--profile")

    command = commands.add_parser("install", help="reference install into a new generation directory")
    command.add_argument("image")
    command.add_argument("dest")
    command.add_argument("--index", required=True)
    command.add_argument("--arch")
    command.add_argument("--profile", default="workspace")

    command = commands.add_parser("fixture", help="write conformance fixtures")
    command.add_argument("out")

    command = commands.add_parser("to-tar", help="rebuild a conventional tar with ownership (podman import)")
    command.add_argument("image")
    command.add_argument("out")
    command.add_argument("--index")
    command.add_argument("--devices", action="store_true")

    command = commands.add_parser("metadata", help="write metadata.base (+ workspace fields) JSON")
    command.add_argument("--versions", required=True)
    command.add_argument("--arch", required=True, choices=sorted(meta.PLATFORMS))
    command.add_argument("--base-out", required=True)
    command.add_argument("--state")
    command.add_argument("--provision")
    command.add_argument("--builder")
    command.add_argument("--extra-out")

    args = parser.parse_args(argv)
    try:
        if args.command == "tree-from-tar":
            warnings = []
            records = tree.from_tar(args.tar, args.out, warnings)
            for warning in warnings:
                print("warning:", warning, file=sys.stderr)
            print(json.dumps({"entries": len(records)}))
        elif args.command == "compare":
            prune = manifest.Prune.load(args.prune_file)
            ignore = [p.encode() for p in args.ignore]
            canonical = manifest.load_canonical(args.canonical_file)
            left = manifest.apply_canonical(_load_manifest(args.left), canonical)
            right = manifest.apply_canonical(_load_manifest(args.right), canonical)
            problems = manifest.compare(left, right, prune, ignore)
            for problem in problems:
                print(*[p.decode("utf-8", "replace") if isinstance(p, bytes) else p for p in problem], sep="\t")
            return 1 if problems else 0
        elif args.command == "pack":
            overrides = dict(item.split("=", 1) for item in args.override)
            index, warnings = pack.pack(
                args.rootfs, manifest.apply_canonical(_load_manifest(args.manifest),
                                                      manifest.load_canonical(args.canonical_file)),
                args.out, prune=manifest.Prune.load(args.prune_file),
                overrides=overrides, compress=not args.no_compress, level=args.level, window_log=args.window_log,
                image_type=args.type, type_version=args.type_version, profile=args.profile,
                architecture=args.arch, base=_load_json(args.base_json), extra=_load_json(args.extra_json))
            for warning in warnings:
                print("warning:", warning, file=sys.stderr)
            print(json.dumps({"sha256": index["sha256"], "size": index["size"],
                              "rootfs": index["metadata"]["rootfs"]}))
        elif args.command == "verify":
            report = verify.verify(args.image, index=_load_json(args.index), architecture=args.arch,
                                   profile=args.profile)
            report.pop("metadata")
            print(json.dumps(report))
        elif args.command == "install":
            report = verify.install(args.image, args.dest, _load_json(args.index), args.arch, args.profile)
            report.pop("metadata")
            print(json.dumps(report))
        elif args.command == "to-tar":
            report = verify.to_tar(args.image, args.out, _load_json(args.index), args.devices)
            report.pop("metadata")
            print(json.dumps(report))
        elif args.command == "metadata":
            versions = meta.read_versions(args.versions)
            with open(args.base_out, "w", encoding="utf-8") as out:
                json.dump(meta.base(versions, args.arch), out)
            if args.extra_out:
                extra = meta.workspace(versions, _load_json(args.state), args.provision, args.builder)
                with open(args.extra_out, "w", encoding="utf-8") as out:
                    json.dump(extra, out)
        elif args.command == "fixture":
            for entry in fixture.generate(args.out):
                print(entry["file"], entry["expect"] or "ok", sep="\t")
    except ImageError as error:
        print("error:", error, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
