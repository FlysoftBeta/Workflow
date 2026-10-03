"""attributes.tsv: the sidecar table carrying ownership, full mode, hardlinks and special files."""
from dataclasses import dataclass

from .common import (ATTRIBUTES_HEADER, MAX_ID, ImageError, check_guest_path, decode_path,
                     encode_path, parent_of)

ROW_TYPES = "dflhcbp"
MEMBER_TYPES = "dfl"


@dataclass(frozen=True)
class Row:
    path: bytes
    type: str
    uid: int
    gid: int
    mode: int
    target: bytes = b""       # primary path for 'h'
    device: tuple = ()        # (major, minor) for 'c'/'b'

    def extra(self):
        if self.type == "h":
            return encode_path(self.target)
        if self.type in "cb":
            return "%d,%d" % self.device
        return "-"


def format_rows(rows):
    lines = [ATTRIBUTES_HEADER]
    for row in rows:
        lines.append(("%s\t%s\t%d\t%d\t%04o\t%s\n" % (
            encode_path(row.path), row.type, row.uid, row.gid, row.mode, row.extra())).encode("ascii"))
    return b"".join(lines)


def _number(text, limit, what):
    if not text.isdigit() or (len(text) > 1 and text[0] == "0"):
        raise ImageError("attributes-syntax", f"bad {what}: {text!r}")
    value = int(text)
    if value > limit:
        raise ImageError("attributes-syntax", f"{what} out of range: {text}")
    return value


def parse_rows(data):
    """Parse the table syntax. Invariants across rows are checked by check_rows()."""
    if not data.startswith(ATTRIBUTES_HEADER):
        raise ImageError("attributes-header", "missing '#workflow-attributes 1' header")
    body = data[len(ATTRIBUTES_HEADER):]
    if body and not body.endswith(b"\n"):
        raise ImageError("attributes-syntax", "table does not end with a newline")
    rows = []
    for number, line in enumerate(body.split(b"\n")[:-1] if body else [], start=2):
        try:
            text = line.decode("ascii")
        except UnicodeDecodeError:
            raise ImageError("attributes-syntax", f"line {number} is not ASCII") from None
        fields = text.split("\t")
        if len(fields) != 6:
            raise ImageError("attributes-syntax", f"line {number} has {len(fields)} fields")
        path_text, kind, uid, gid, mode, extra = fields
        if kind not in ROW_TYPES or len(kind) != 1:
            raise ImageError("attributes-syntax", f"line {number}: unknown type {kind!r}")
        if len(mode) != 4 or any(c not in "01234567" for c in mode):
            raise ImageError("attributes-syntax", f"line {number}: bad mode {mode!r}")
        target, device = b"", ()
        if kind == "h":
            target = check_guest_path(decode_path(extra))
        elif kind in "cb":
            parts = extra.split(",")
            if len(parts) != 2:
                raise ImageError("attributes-syntax", f"line {number}: bad device {extra!r}")
            device = (_number(parts[0], MAX_ID, "major"), _number(parts[1], MAX_ID, "minor"))
        elif extra != "-":
            raise ImageError("attributes-syntax", f"line {number}: unexpected extra {extra!r}")
        row = Row(check_guest_path(decode_path(path_text)), kind, _number(uid, MAX_ID, "uid"),
                  _number(gid, MAX_ID, "gid"), int(mode, 8), target, device)
        if kind == "l" and row.mode != 0o777:
            raise ImageError("attributes-syntax", f"line {number}: symlink mode must be 0777")
        rows.append(row)
    return rows


def check_rows(rows):
    """Check invariants 1, 2 and 4 of docs/environment.md §2.4. Returns summary counts."""
    if not rows or rows[0].path != b"/" or rows[0].type != "d":
        raise ImageError("attributes-root", "first row must be the root directory")
    kinds = {}
    previous = None
    counts = {"rows": 0, "members": 0, "hardlinks": 0, "special": 0}
    for row in rows:
        if previous is not None and row.path <= previous:
            raise ImageError("row-order", "rows are not strictly sorted at " + encode_path(row.path))
        previous = row.path
        parent = parent_of(row.path)
        if parent is not None and kinds.get(parent, (None,))[0] != "d":
            raise ImageError("parent-not-dir", encode_path(row.path))
        if row.type == "h":
            primary = kinds.get(row.target)
            if primary is None or primary[0] != "f":
                raise ImageError("hardlink-primary", encode_path(row.path) + " -> " + encode_path(row.target))
            if primary[1:] != (row.uid, row.gid, row.mode):
                raise ImageError("hardlink-attributes", encode_path(row.path))
            counts["hardlinks"] += 1
        if row.type in "cbp":
            counts["special"] += 1
        if row.type in MEMBER_TYPES:
            counts["members"] += 1
        kinds[row.path] = (row.type, row.uid, row.gid, row.mode)
        counts["rows"] += 1
    return counts
