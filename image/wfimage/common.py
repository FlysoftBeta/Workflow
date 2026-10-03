"""Shared constants and helpers for the Workflow image format (docs/engine/environment.md §2)."""
import stat

FORMAT = "workflow-image"
FORMAT_VERSION = 2
ATTRIBUTES_NAME = "attributes.tsv"
ATTRIBUTES_VERSION = 1
ATTRIBUTES_HEADER = b"#workflow-attributes 1\n"
METADATA_NAME = "metadata.json"
ROOTFS_PREFIX = "rootfs"
METADATA_LIMIT = 64 * 1024
ATTRIBUTES_LIMIT = 64 * 1024 * 1024
KNOWN_TYPES = {"debian-trixie": {1}}
ARCHITECTURES = {"arm64", "amd64"}
PROFILES = {"workspace", "base"}
REQUIRES = ("virtual-ownership", "virtual-mode", "hardlink-emulation", "virtual-special-files", "store-seeds")
SEED_POLICIES = {"if-absent", "merge"}
ALWAYS_REQUIRED = ("virtual-ownership", "virtual-mode")
ALLOWED_PAX_KEYS = {"path", "linkpath", "size", "mtime"}
MAX_ID = 4294967294


class ImageError(Exception):
    """A violation of the image contract. `code` is stable and used by fixtures and tests."""

    def __init__(self, code, message):
        super().__init__(f"{code}: {message}")
        self.code = code


def encode_path(raw):
    """Percent-encode a guest path: 0x21-0x7E except '%' stay, every other byte becomes %XX."""
    return "".join(chr(b) if 0x21 <= b <= 0x7E and b != 0x25 else "%%%02X" % b for b in raw)


def decode_path(text):
    """Strict inverse of encode_path; rejects non-canonical encodings such as %41 or lowercase hex."""
    out = bytearray()
    index = 0
    while index < len(text):
        char = text[index]
        if char == "%":
            digits = text[index + 1:index + 3]
            if len(digits) != 2 or any(c not in "0123456789ABCDEF" for c in digits):
                raise ImageError("attributes-syntax", "bad percent escape in " + text)
            value = int(digits, 16)
            if 0x21 <= value <= 0x7E and value != 0x25:
                raise ImageError("attributes-syntax", "non-canonical escape in " + text)
            out.append(value)
            index += 3
        elif 0x21 <= ord(char) <= 0x7E:
            out.append(ord(char))
            index += 1
        else:
            raise ImageError("attributes-syntax", "unencoded byte in path")
    return bytes(out)


def check_guest_path(raw):
    """Validate an absolute, normalized, UTF-8 guest path (bytes)."""
    if raw == b"/":
        return raw
    if not raw.startswith(b"/") or raw.endswith(b"/") or b"\0" in raw:
        raise ImageError("unsafe-path", repr(raw))
    for part in raw[1:].split(b"/"):
        if part in (b"", b".", b".."):
            raise ImageError("unsafe-path", repr(raw))
    try:
        raw.decode("utf-8")
    except UnicodeDecodeError:
        raise ImageError("non-utf8-path", repr(raw)) from None
    return raw


def parent_of(raw):
    if raw == b"/":
        return None
    head = raw.rsplit(b"/", 1)[0]
    return head or b"/"


def member_name(raw):
    """Tar member name for a guest path."""
    return ROOTFS_PREFIX if raw == b"/" else ROOTFS_PREFIX + raw.decode("utf-8")


def guest_from_member(name):
    """Inverse of member_name; the caller has already stripped one trailing '/'."""
    if name == ROOTFS_PREFIX:
        return b"/"
    if not name.startswith(ROOTFS_PREFIX + "/"):
        raise ImageError("unexpected-member", name)
    return check_guest_path(name[len(ROOTFS_PREFIX):].encode("utf-8"))


TYPE_OF_MODE = {
    stat.S_IFDIR: "d", stat.S_IFREG: "f", stat.S_IFLNK: "l",
    stat.S_IFCHR: "c", stat.S_IFBLK: "b", stat.S_IFIFO: "p", stat.S_IFSOCK: "s",
}


def type_of_mode(raw_mode):
    kind = TYPE_OF_MODE.get(stat.S_IFMT(raw_mode))
    if kind is None:
        raise ImageError("unknown-type", oct(raw_mode))
    return kind
