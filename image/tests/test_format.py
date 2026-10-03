"""Host tests for the image format tooling. Run: python3 -m unittest discover -s image/tests"""
import io
import json
import os
import socket
import stat
import subprocess
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path

IMAGE_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(IMAGE_DIR))

from wfimage import attributes, fixture, manifest, pack, tarstream, tree, verify  # noqa: E402
from wfimage.attributes import Row  # noqa: E402
from wfimage.common import ImageError, decode_path, encode_path  # noqa: E402

PACK_ARGS = dict(image_type="debian-trixie", type_version=1, profile="workspace", architecture="amd64",
                 base=fixture.BASE, extra=fixture.EXTRA)


class Scratch(unittest.TestCase):
    def setUp(self):
        self._dir = tempfile.TemporaryDirectory()
        self.dir = Path(self._dir.name)
        os.environ["SOURCE_DATE_EPOCH"] = "1700000000"

    def tearDown(self):
        self._dir.cleanup()

    def assertCode(self, code, function, *args, **kwargs):
        with self.assertRaises(ImageError) as caught:
            function(*args, **kwargs)
        self.assertEqual(caught.exception.code, code, str(caught.exception))


class PathCodecTests(unittest.TestCase):
    def test_round_trip_and_canonical_form(self):
        raw = "/etc/ssl/证书 %\t.pem".encode()
        text = encode_path(raw)
        self.assertEqual(text, "/etc/ssl/%E8%AF%81%E4%B9%A6%20%25%09.pem")
        self.assertEqual(decode_path(text), raw)
        for bad in ("/a%41", "/a%e8", "/a%2", "/a b", "/aé"):
            with self.assertRaises(ImageError):
                decode_path(bad)


class AttributesTests(unittest.TestCase):
    ROWS = [Row(b"/", "d", 0, 0, 0o755), Row(b"/bin", "l", 0, 0, 0o777), Row(b"/dev", "d", 0, 0, 0o755),
            Row(b"/dev/null", "c", 0, 0, 0o666, device=(1, 3)), Row(b"/usr", "d", 0, 0, 0o755),
            Row(b"/usr/perl", "f", 0, 0, 0o755), Row(b"/usr/perl5", "h", 0, 0, 0o755, target=b"/usr/perl"),
            Row(b"/usr/sudo", "f", 0, 0, 0o4755)]

    def test_round_trip(self):
        data = attributes.format_rows(self.ROWS)
        self.assertEqual(attributes.parse_rows(data), self.ROWS)
        self.assertEqual(attributes.check_rows(self.ROWS),
                         {"rows": 8, "members": 6, "hardlinks": 1, "special": 1})
        self.assertIn(b"/usr/sudo\tf\t0\t0\t4755\t-\n", data)

    def test_syntax_errors(self):
        header = b"#workflow-attributes 1\n"
        for body in (b"/\td\t0\t0\t755\t-\n", b"/\td\t0\t0\t0755\n", b"/\tx\t0\t0\t0755\t-\n",
                     b"/\td\t01\t0\t0755\t-\n", b"/\td\t0\t0\t0755\tjunk\n", b"/\td\t0\t0\t0755\t-",
                     b"/l\tl\t0\t0\t0755\t-\n", b"/c\tc\t0\t0\t0644\t1\n"):
            with self.assertRaises(ImageError, msg=body):
                attributes.parse_rows(header + body)
        with self.assertRaises(ImageError):
            attributes.parse_rows(b"#workflow-attributes 2\n")

    def test_invariants(self):
        cases = {
            "attributes-root": [Row(b"/a", "d", 0, 0, 0o755)],
            "row-order": [Row(b"/", "d", 0, 0, 0o755), Row(b"/b", "d", 0, 0, 0o755), Row(b"/a", "d", 0, 0, 0o755)],
            "parent-not-dir": [Row(b"/", "d", 0, 0, 0o755), Row(b"/a", "l", 0, 0, 0o777),
                               Row(b"/a/b", "f", 0, 0, 0o644)],
            "hardlink-primary": [Row(b"/", "d", 0, 0, 0o755), Row(b"/a", "h", 0, 0, 0o644, target=b"/b"),
                                 Row(b"/b", "f", 0, 0, 0o644)],
            "hardlink-attributes": [Row(b"/", "d", 0, 0, 0o755), Row(b"/a", "f", 0, 0, 0o644),
                                    Row(b"/b", "h", 1, 0, 0o644, target=b"/a")],
        }
        for code, rows in cases.items():
            with self.assertRaises(ImageError) as caught:
                attributes.check_rows(rows)
            self.assertEqual(caught.exception.code, code)


class TarStreamTests(unittest.TestCase):
    def test_pax_names_links_and_gnu_tar_interop(self):
        out = io.BytesIO()
        writer = tarstream.Writer(out)
        long_name = "rootfs/" + "d" * 150 + "/证书.pem"
        writer.member("rootfs", tarstream.DIR, mode=0o755)
        writer.member(long_name, tarstream.REG, b"pem", mode=0o644, mtime=123)
        writer.member("rootfs/link", tarstream.SYM, linkname="/" + "t" * 120)
        writer.close()
        reader = tarstream.Reader(io.BytesIO(out.getvalue()))
        names = []
        while (member := reader.next()) is not None:
            names.append((member.name, member.linkname, reader.read_data()))
        self.assertEqual(names, [("rootfs", "", b""), (long_name, "", b"pem"), ("rootfs/link", "/" + "t" * 120, b"")])
        with tarfile.open(fileobj=io.BytesIO(out.getvalue())) as archive:
            self.assertEqual(archive.getnames(), ["rootfs", long_name, "rootfs/link"])

    def test_rejects_checksum_and_gnu_headers(self):
        out = io.BytesIO()
        writer = tarstream.Writer(out)
        writer.member("a", tarstream.REG, b"x")
        writer.close()
        corrupt = bytearray(out.getvalue())
        corrupt[0] = ord("b")
        with self.assertRaises(ImageError):
            tarstream.Reader(io.BytesIO(bytes(corrupt))).next()
        gnu = tarstream.header(b"a", b"0", magic=b"ustar  \0") + b"\0" * 1024
        with self.assertRaises(ImageError):
            tarstream.Reader(io.BytesIO(gnu)).next()


class FixtureTests(Scratch):
    def test_every_fixture_behaves_as_listed(self):
        listing = fixture.generate(self.dir)
        self.assertGreaterEqual(len([e for e in listing if e["expect"]]), 20)
        for entry in listing:
            path = self.dir / entry["file"]
            index = json.loads((self.dir / entry["index"]).read_text()) if entry.get("index") else None
            if entry["expect"] is None:
                report = verify.verify(path, index=index, architecture="amd64", profile="workspace")
                self.assertEqual((report["rows"], report["hardlinks"], report["special"]), (36, 2, 2))
            else:
                self.assertCode(entry["expect"], verify.verify, path)

    def test_compressed_and_plain_fixtures_are_the_same_deterministic_tar(self):
        fixture.generate(self.dir)
        plain = (self.dir / "good.tar").read_bytes()
        unpacked = subprocess.run(["zstd", "-dc", str(self.dir / "good.tar.zst")], capture_output=True,
                                  check=True).stdout
        self.assertEqual(plain, unpacked)
        second = self.dir / "again"
        fixture.generate(second)
        self.assertEqual((second / "good.tar").read_bytes(), plain)

    def test_architecture_and_profile_are_enforced(self):
        fixture.generate(self.dir)
        self.assertCode("architecture", verify.verify, self.dir / "good.tar", architecture="arm64")
        self.assertCode("profile", verify.verify, self.dir / "good.tar", profile="base")
        self.assertCode("unsupported-requirement", verify.verify, self.dir / "good.tar",
                        capabilities=("virtual-ownership", "virtual-mode"))


class PackTests(Scratch):
    def tree(self):
        rootfs, records = fixture.build_tree(self.dir / "src")
        return rootfs, records

    def test_unlisted_missing_and_mismatched_entries_fail(self):
        rootfs, records = self.tree()
        (Path(os.fsdecode(rootfs)) / "etc/extra").write_text("x")
        self.assertCode("unlisted", pack.build_plan, rootfs, records)
        os.remove(Path(os.fsdecode(rootfs)) / "etc/extra")
        os.remove(Path(os.fsdecode(rootfs)) / "usr/share/empty")
        self.assertCode("missing", pack.build_plan, rootfs, records)
        os.mkdir(Path(os.fsdecode(rootfs)) / "usr/share/empty")
        self.assertCode("type-mismatch", pack.build_plan, rootfs, records)

    def test_prune_override_and_sockets(self):
        rootfs, records = self.tree()
        root = Path(os.fsdecode(rootfs))
        (root / "tmp/scratch").write_text("junk")
        server = socket.socket(socket.AF_UNIX)
        cwd = os.getcwd()
        os.chdir(root / "run")   # AF_UNIX paths are limited to 108 bytes
        try:
            server.bind("control.sock")
        finally:
            os.chdir(cwd)
        prune = manifest.Prune(["./tmp/*"])
        placeholder = self.dir / "shadow"
        placeholder.write_bytes(b"root:!:1::::::\n")
        plan = pack.build_plan(rootfs, records, prune, {"/etc/shadow": str(placeholder)})
        server.close()
        self.assertIn("dropped socket /run/control.sock", plan.warnings)
        self.assertNotIn(b"/tmp/scratch", [r.path for r in plan.rows])
        self.assertEqual(plan.sources[b"/etc/shadow"][1], len(placeholder.read_bytes()))
        os.remove(root / "run/control.sock")
        self.assertCode("override", pack.build_plan, rootfs, records, prune, {"/usr/bin/perl5.40": str(placeholder)})
        self.assertCode("override", pack.build_plan, rootfs, records, prune, {"/nope": str(placeholder)})

    def test_non_utf8_names_are_rejected(self):
        rootfs, records = self.tree()
        bad = rootfs + b"/etc/\xff"
        with open(bad, "wb"):
            pass
        records[b"/etc/\xff"] = manifest.Record(stat.S_IFREG | 0o644, 0, 0, "x", 1, (0, 0), b"/etc/\xff")
        self.assertCode("non-utf8-path", pack.build_plan, rootfs, records)

    def test_workspace_metadata_requires_workspace_fields(self):
        rootfs, records = self.tree()
        args = dict(PACK_ARGS, extra={})
        self.assertCode("metadata", pack.pack, rootfs, records, self.dir / "x.tar", compress=False, **args)

    def test_metadata_and_index(self):
        rootfs, records = self.tree()
        index, _ = pack.pack(rootfs, records, self.dir / "image.tar.zst", level=1, **PACK_ARGS)
        on_disk = json.loads((self.dir / "image.json").read_text())
        self.assertEqual(on_disk, index)
        metadata = index["metadata"]
        self.assertEqual(metadata["requires"], ["virtual-ownership", "virtual-mode", "hardlink-emulation",
                                                "virtual-special-files", "store-seeds"])
        self.assertEqual(metadata["rootfs"], {"entries": 36, "members": 32, "regularBytes": 107})
        self.assertEqual(metadata["createdAt"], "2023-11-14T22:13:20Z")


class InstallTests(Scratch):
    def setUp(self):
        super().setUp()
        fixture.generate(self.dir / "fx")
        self.image = self.dir / "fx/good.tar.zst"
        self.index = json.loads((self.dir / "fx/good.json").read_text())

    def test_reference_install(self):
        target = self.dir / "generations/1"
        verify.install(self.image, target, self.index, "amd64")
        rootfs = target / "rootfs"
        self.assertEqual((rootfs / "etc/ssl/证书.pem").read_bytes(), b"-----BEGIN CERTIFICATE-----\n")
        sudo = os.stat(rootfs / "usr/bin/sudo")
        self.assertEqual(stat.S_IMODE(sudo.st_mode) & 0o7000, 0, "setuid must stay virtual")
        self.assertEqual(os.stat(rootfs / "usr/bin/perl").st_ino, os.stat(rootfs / "usr/lib/perl-alias").st_ino)
        self.assertEqual(os.readlink(rootfs / "var/absolute"), "/etc/shadow")
        self.assertEqual(os.lstat(rootfs / "etc/shadow").st_mtime, 1700000000)
        self.assertFalse((rootfs / "dev/null").exists() or (rootfs / "run/initctl").exists())
        self.assertIn(b"/dev/null\tc\t0\t0\t0666\t1,3\n", (target / "attributes.tsv").read_bytes())
        # /home/work is a store: its content is seeded with real permission bits, the prefix stays as a mount point.
        seed = target / "seeds/home/work"
        self.assertEqual((seed / ".profile").read_bytes(), b"export EDITOR=nano\n")
        self.assertEqual(stat.S_IMODE(os.stat(seed / "space %\ttab").st_mode), 0o600)
        self.assertEqual(list((rootfs / "home/work").iterdir()), [])
        self.assertFalse(Path(str(target) + ".partial").exists())
        self.assertCode("exists", verify.install, self.image, target, self.index)

    def test_failed_install_leaves_nothing(self):
        target = self.dir / "generations/2"
        self.assertCode("member-order", verify.install, self.dir / "fx/bad-member-order.tar", target, None)
        self.assertFalse(target.exists() or Path(str(target) + ".partial").exists())
        tampered = dict(self.index, sha256="0" * 64)
        self.assertCode("image-digest", verify.install, self.image, target, tampered)
        self.assertFalse(target.exists())

    def test_symlink_parent_is_never_followed(self):
        outside = self.dir / "outside"
        outside.mkdir()
        self.assertCode("parent-not-dir", verify.install, self.dir / "fx/bad-parent-not-dir.tar",
                        self.dir / "g3", None)
        self.assertEqual(list(outside.iterdir()), [])


class TreeTests(Scratch):
    def export(self, add):
        path = self.dir / "export.tar"
        with tarfile.open(path, "w", format=tarfile.PAX_FORMAT) as archive:
            for name, kind, extra in add:
                info = tarfile.TarInfo(name)
                info.type = kind
                data = None
                for key, value in extra.items():
                    if key == "data":
                        data = io.BytesIO(value)
                        info.size = len(value)
                    else:
                        setattr(info, key, value)
                archive.addfile(info, data)
        return path

    def test_rootless_conversion_records_everything(self):
        path = self.export([
            ("./", tarfile.DIRTYPE, {"mode": 0o755}),
            ("./usr", tarfile.DIRTYPE, {"mode": 0o755}),
            ("./usr/bin/sudo", tarfile.REGTYPE, {"mode": 0o4755, "data": b"elf", "mtime": 1234}),
            ("./usr/bin/sudoedit", tarfile.LNKTYPE, {"linkname": "./usr/bin/sudo"}),
            ("./etc/证书.pem", tarfile.REGTYPE, {"mode": 0o644, "uid": 0, "gid": 42, "data": b"pem"}),
            ("./bin", tarfile.SYMTYPE, {"linkname": "usr/bin"}),
            ("./dev/null", tarfile.CHRTYPE, {"mode": 0o666, "devmajor": 1, "devminor": 3}),
            ("./run/fifo", tarfile.FIFOTYPE, {"mode": 0o600}),
            ("./home/work", tarfile.DIRTYPE, {"mode": 0o700, "uid": 1000, "gid": 1000}),
        ])
        warnings = []
        records = tree.from_tar(str(path), self.dir / "tree", warnings)
        root = self.dir / "tree/rootfs"
        self.assertEqual((root / "usr/bin/sudo").read_bytes(), b"elf")
        self.assertEqual(os.stat(root / "usr/bin/sudo").st_ino, os.stat(root / "usr/bin/sudoedit").st_ino)
        self.assertFalse(os.path.lexists(root / "dev/null"))
        self.assertEqual(records[b"/usr/bin/sudo"].mode & 0o7777, 0o4755)
        self.assertEqual(records[b"/usr/bin/sudoedit"].key, records[b"/usr/bin/sudo"].key)
        self.assertEqual(records[b"/usr/bin/sudo"].nlink, 2)
        self.assertEqual(records[b"/dev/null"].device, (1, 3))
        self.assertEqual(records["/etc/证书.pem".encode()].gid, 42)
        self.assertIn("implicit directory /usr/bin", warnings)
        self.assertEqual(os.stat(root / "usr/bin/sudo").st_mtime, 1234)
        parsed = manifest.parse((self.dir / "tree/manifest").read_bytes())
        self.assertEqual(parsed, records)

    def test_hostile_tars_are_rejected(self):
        cases = {
            "unsafe-path": [("../escape", tarfile.REGTYPE, {"data": b"x"})],
            "parent-not-dir": [("link", tarfile.SYMTYPE, {"linkname": "/tmp"}),
                               ("link/evil", tarfile.REGTYPE, {"data": b"x"})],
            "manifest-duplicate": [("a", tarfile.REGTYPE, {"data": b"x"}), ("a", tarfile.REGTYPE, {"data": b"y"})],
            "hardlink-primary": [("a", tarfile.LNKTYPE, {"linkname": "missing"})],
        }
        for index, (code, entries) in enumerate(cases.items()):
            path = self.export(entries)
            self.assertCode(code, tree.from_tar, str(path), self.dir / f"t{index}")


class ManifestTests(Scratch):
    def test_capture_script_matches_lstat_and_prune(self):
        root = self.dir / "root"
        (root / "proc/1").mkdir(parents=True)
        (root / "run/lock/x").mkdir(parents=True)
        (root / "etc").mkdir()
        (root / "etc/a b").write_text("a")
        os.link(root / "etc/a b", root / "etc/hard")
        os.symlink("/etc/a b", root / "etc/sym")
        output = subprocess.run([str(IMAGE_DIR / "guest/capture-manifest.sh"), str(root)], capture_output=True,
                                check=True).stdout
        records = manifest.parse(output)
        self.assertEqual(sorted(records), [b"/", b"/etc", b"/etc/a b", b"/etc/hard", b"/etc/sym", b"/proc",
                                           b"/run", b"/run/lock"])
        self.assertEqual(records[b"/etc/a b"].key, records[b"/etc/hard"].key)
        self.assertEqual(records[b"/etc/hard"].nlink, 2)
        self.assertEqual(records[b"/etc/sym"].type, "l")
        self.assertEqual(records[b"/etc"].uid, os.getuid())
        index, _ = pack.pack(str(root), records, self.dir / "img.tar", compress=False,
                             prune=manifest.Prune.load(IMAGE_DIR / "guest/prune.list"),
                             **dict(PACK_ARGS, profile="base", extra={}))
        report = verify.verify(self.dir / "img.tar", index=index)
        self.assertEqual(report["hardlinks"], 1)

    def test_compare_reports_differences(self):
        rootfs, records = fixture.build_tree(self.dir / "src")
        self.assertEqual(manifest.compare(records, dict(records)), [])
        changed = dict(records)
        sudo = changed[b"/usr/bin/sudo"]
        changed[b"/usr/bin/sudo"] = manifest.Record(sudo.mode & ~0o4000, sudo.uid, sudo.gid, sudo.key, 1, (0, 0),
                                                    sudo.path)
        del changed[b"/var/mail"]
        problems = manifest.compare(records, changed)
        self.assertIn(("mode", b"/usr/bin/sudo", "0o4755", "0o755"), problems)
        self.assertIn(("only-left", b"/var/mail"), problems)
        self.assertEqual(manifest.compare(records, changed, ignore=[b"/usr/bin/sudo", b"/var/mail"]), [])


class StoreTests(Scratch):
    def test_store_rules(self):
        from wfimage import stores
        parsed = stores.parse({"stores": {"/home/work": {"store": "home/work", "seed": "if-absent"}}})
        self.assertEqual(parsed, {b"/home/work": ("home/work", "if-absent")})
        for bad in ({"/": {"store": "x", "seed": "merge"}}, {"/a": {"store": "../x", "seed": "merge"}},
                    {"/a": {"store": "x", "seed": "replace"}},
                    {"/a": {"store": "x", "seed": "merge"}, "/a/b": {"store": "y", "seed": "merge"}}):
            self.assertCode("metadata", stores.parse, {"stores": bad})
        rows = [Row(b"/", "d", 0, 0, 0o755), Row(b"/opt", "d", 0, 0, 0o755), Row(b"/opt/t", "d", 1000, 1000, 0o755),
                Row(b"/opt/t/bin", "f", 1000, 1000, 0o755), Row(b"/opt/t/x", "h", 1000, 1000, 0o755, target=b"/opt/t/bin")]
        rules = {b"/opt/t": ("toolchains", "merge")}
        self.assertCode("store-seed", stores.check_rows, rows, rules, (1000, 1000))
        self.assertEqual(stores.check_rows(rows[:4], rules, (1000, 1000)), 1)
        self.assertCode("store-seed", stores.check_rows, rows[:4], rules, (0, 0))
        self.assertCode("store-seed", stores.check_rows, rows[:1], rules, (1000, 1000))
        setuid = rows[:3] + [Row(b"/opt/t/bin", "f", 1000, 1000, 0o4755)]
        self.assertCode("store-seed", stores.check_rows, setuid, rules, (1000, 1000))


class CanonicalAndMetaTests(Scratch):
    def test_canonical_list_normalises_mount_points(self):
        canonical = manifest.load_canonical(IMAGE_DIR / "guest/canonical.list")
        proc = manifest.Record(stat.S_IFDIR | 0o555, 65534, 65534, "p", 1, (0, 0), b"/proc")
        hosts = manifest.Record(stat.S_IFREG | 0o700, 0, 0, "h", 1, (0, 0), b"/etc/hosts")
        result = manifest.apply_canonical({b"/proc": proc, b"/etc/hosts": hosts}, canonical)
        self.assertEqual((result[b"/proc"].uid, result[b"/proc"].mode), (0, stat.S_IFDIR | 0o555))
        self.assertEqual(result[b"/etc/hosts"].mode, stat.S_IFREG | 0o644)
        self.assertNotIn(b"/sys", result)
        wrong = manifest.Record(stat.S_IFLNK | 0o777, 0, 0, "x", 1, (0, 0), b"/proc")
        self.assertCode("canonical", manifest.apply_canonical, {b"/proc": wrong}, canonical)

    def test_metadata_from_versions_env(self):
        from wfimage import meta
        versions = meta.read_versions(IMAGE_DIR / "versions.env")
        base = meta.base(versions, "arm64")
        self.assertEqual(base["platform"], "linux/arm64/v8")
        self.assertTrue(base["manifestDigest"].startswith("sha256:"))
        self.assertIn("build-essential", versions["PACKAGES"].split())
        extra = meta.workspace(versions, {"python": "3.14.7", "node": "24.21.0", "profile": "py3.14.7-node24.21.0"},
                               IMAGE_DIR / "guest/provision.sh", "podman")
        self.assertEqual(set(extra["stores"]), {"/home/work", "/opt/toolchains"})
        self.assertEqual(extra["toolchains"]["profile"], "py3.14.7-node24.21.0")
        self.assertEqual(extra["defaults"], {"python": versions["DEFAULT_PYTHON"], "node": versions["DEFAULT_NODE"]})
        self.assertTrue(extra["environment"]["PATH"].startswith("/opt/toolchains/active/python/bin:"))
        self.assertEqual(extra["environment"]["UV_PYTHON_PREFERENCE"], "system")
        with self.assertRaises(ValueError):
            meta.workspace(versions, {"python": None, "node": "24"}, IMAGE_DIR / "guest/provision.sh", "x")

    def test_to_tar_restores_ownership_and_links(self):
        fixture.generate(self.dir / "fx")
        verify.to_tar(self.dir / "fx/good.tar.zst", self.dir / "plain.tar")
        with tarfile.open(self.dir / "plain.tar") as archive:
            members = {m.name: m for m in archive.getmembers()}
        self.assertEqual((members["./usr/bin/sudo"].mode, members["./etc/shadow"].gid), (0o4755, 42))
        self.assertTrue(members["./usr/lib/perl-alias"].islnk())
        self.assertEqual(members["./usr/lib/perl-alias"].linkname, "./usr/bin/perl")
        self.assertTrue(members["./run/initctl"].isfifo())
        self.assertNotIn("./dev/null", members)
        self.assertCode("member-order", verify.to_tar, self.dir / "fx/bad-member-order.tar", self.dir / "bad.tar")
        self.assertFalse((self.dir / "bad.tar").exists() or (self.dir / "bad.tar.partial").exists())


class ScriptTests(unittest.TestCase):
    def test_guest_scripts_parse(self):
        for name in ("guest/provision.sh", "guest/envctl", "guest/capture-manifest.sh", "guest/pack-in-guest.sh",
                     "build.sh", "with-cross.sh", "tests/podman-roundtrip.sh", "tests/toolchain-profiles.sh"):
            subprocess.run(["bash", "-n", str(IMAGE_DIR / name)], check=True)

    def test_envctl_rejects_bad_input_without_side_effects(self):
        envctl = str(IMAGE_DIR / "guest/envctl")
        for args in (["bogus"], ["python", "2.7"], ["node", "lts/*"], ["state", "Bad_Name"]):
            result = subprocess.run([envctl, *args], capture_output=True)
            self.assertEqual(result.returncode, 2, args)
        result = subprocess.run([envctl, "state"], capture_output=True, check=True)
        self.assertEqual(json.loads(result.stdout), {"profile": None, "python": None, "node": None, "installedPython": [], "installedNode": [], "packages": {}})
        self.assertEqual(subprocess.run([envctl, "activate", "../evil"], capture_output=True).returncode, 2)


if __name__ == "__main__":
    unittest.main()
