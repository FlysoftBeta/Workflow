"""Minimal strict POSIX tar (ustar + PAX) writer and reader for the image format.

The reader mirrors what the engine's installer must accept (docs/environment.md §2.2) and rejects
everything else; it deliberately does not use `tarfile`, which silently accepts GNU extensions.
"""
from dataclasses import dataclass, field

from .common import ALLOWED_PAX_KEYS, ImageError

BLOCK = 512
REG, DIR, SYM, PAX = b"0", b"5", b"2", b"x"
_OCTAL_LIMIT = {8: 0o7777777, 12: 0o77777777777}


def _octal(value, width):
    if value < 0 or value > _OCTAL_LIMIT[width]:
        raise ValueError("value does not fit")
    return b"%0*o\0" % (width - 1, value)


def _pax_record(key, value):
    payload = b" " + key.encode("utf-8") + b"=" + value + b"\n"
    length = len(payload) + 1
    while len(b"%d" % length) + len(payload) != length:
        length = len(b"%d" % length) + len(payload)
    return b"%d" % length + payload


def header(name, kind, size=0, mode=0, mtime=0, linkname=b"", uid=0, gid=0, magic=b"ustar\x0000"):
    block = bytearray(BLOCK)
    block[0:len(name)] = name
    block[100:108] = _octal(mode, 8)
    block[108:116] = _octal(uid, 8)
    block[116:124] = _octal(gid, 8)
    block[124:136] = _octal(size, 12)
    block[136:148] = _octal(mtime, 12)
    block[148:156] = b" " * 8
    block[156:157] = kind
    block[157:157 + len(linkname)] = linkname
    block[257:265] = magic
    block[329:337] = _octal(0, 8)
    block[337:345] = _octal(0, 8)
    block[148:156] = b"%06o\0 " % sum(block)
    return bytes(block)


def padding(size):
    return b"\0" * (-size % BLOCK)


class Writer:
    """Writes members; long/non-ASCII names, links, large sizes and odd mtimes go into PAX headers."""

    def __init__(self, out):
        self.out = out

    def member(self, name, kind, data=b"", size=None, mode=0o644, mtime=0, linkname="", source=None):
        raw_name, raw_link = name.encode("utf-8"), linkname.encode("utf-8")
        size = len(data) if size is None else size
        pax = []
        if len(raw_name) > 100 or not raw_name.isascii():
            pax.append(_pax_record("path", raw_name))
            raw_name = raw_name[:100] if raw_name.isascii() else b"@PaxPath"
        if len(raw_link) > 100 or not raw_link.isascii():
            pax.append(_pax_record("linkpath", raw_link))
            raw_link = b""
        if size > _OCTAL_LIMIT[12]:
            pax.append(_pax_record("size", b"%d" % size))
        if not 0 <= mtime <= _OCTAL_LIMIT[12]:
            pax.append(_pax_record("mtime", b"%d" % mtime))
            mtime = 0
        if pax:
            body = b"".join(pax)
            self.out.write(header(b"@PaxHeader", PAX, len(body)) + body + padding(len(body)))
        self.out.write(header(raw_name, kind, size if kind == REG else 0, mode & 0o777,
                              mtime, raw_link if kind == SYM else b""))
        if kind == REG:
            written = 0
            if source is None:
                self.out.write(data)
                written = len(data)
            else:
                while chunk := source.read(1 << 20):
                    self.out.write(chunk)
                    written += len(chunk)
            if written != size:
                raise ImageError("size-changed", f"{name}: expected {size} bytes, read {written}")
            self.out.write(padding(size))

    def close(self):
        self.out.write(b"\0" * (BLOCK * 2))


@dataclass
class Member:
    name: str
    kind: bytes
    size: int
    mode: int
    mtime: int
    linkname: str
    pax: dict = field(default_factory=dict)


def _parse_octal(raw, what):
    raw = raw.rstrip(b"\0 ").lstrip(b" ")
    if raw and raw[0] & 0x80:
        raise ImageError("tar-header", f"base-256 {what} not allowed")
    if raw and any(c not in b"01234567" for c in raw):
        raise ImageError("tar-header", f"bad octal {what}")
    return int(raw, 8) if raw else 0


def _parse_pax(body):
    values, index = {}, 0
    while index < len(body):
        space = body.find(b" ", index)
        if space < 0:
            raise ImageError("tar-pax", "bad PAX record")
        length = int(body[index:space])
        record = body[index:index + length]
        if length <= 0 or not record.endswith(b"\n") or b"=" not in record:
            raise ImageError("tar-pax", "bad PAX record")
        key, value = record[space - index + 1:-1].split(b"=", 1)
        key = key.decode("utf-8")
        if key not in ALLOWED_PAX_KEYS:
            raise ImageError("tar-pax", f"PAX keyword {key!r} not allowed")
        values[key] = value.decode("utf-8")
        index += length
    return values


class Reader:
    """Sequential reader over a binary stream. Iterate members, then call read_data()/skip_data()."""

    def __init__(self, stream):
        self.stream, self.pending, self.remaining = stream, 0, 0
        self.ended = False

    def _read_exact(self, count):
        chunks, remaining = [], count
        while remaining:
            chunk = self.stream.read(remaining)
            if not chunk:
                raise ImageError("tar-truncated", "unexpected end of archive")
            chunks.append(chunk)
            remaining -= len(chunk)
        return b"".join(chunks)

    def _block(self):
        return self._read_exact(BLOCK)

    def next(self):
        self.skip_data()
        pax = {}
        while True:
            block = self._block()
            if block == b"\0" * BLOCK:
                if self._block() != b"\0" * BLOCK:
                    raise ImageError("tar-end", "single zero block")
                while chunk := self.stream.read(BLOCK):
                    if chunk.strip(b"\0"):
                        raise ImageError("tar-end", "data after end of archive")
                self.ended = True
                return None
            stored = _parse_octal(block[148:156], "checksum")
            if stored != sum(block[:148]) + 256 + sum(block[156:]):
                raise ImageError("tar-header", "checksum mismatch")
            if block[257:265] != b"ustar\x0000":
                raise ImageError("tar-header", "not a POSIX ustar header")
            kind = block[156:157]
            size = _parse_octal(block[124:136], "size")
            if kind == PAX:
                if pax:
                    raise ImageError("tar-pax", "consecutive PAX headers")
                body = self._read_exact(size + (-size % BLOCK))[:size]
                pax = _parse_pax(body)
                if not pax:
                    raise ImageError("tar-pax", "empty PAX header")
                continue
            if kind not in (REG, DIR, SYM):
                raise ImageError("forbidden-type", "tar member type %r" % kind.decode("latin-1"))
            name = block[0:100].split(b"\0", 1)[0]
            prefix = block[345:500].split(b"\0", 1)[0]
            if prefix:
                name = prefix + b"/" + name
            try:
                text = pax.get("path") or name.decode("utf-8")
                link = pax.get("linkpath") or block[157:257].split(b"\0", 1)[0].decode("utf-8")
            except UnicodeDecodeError:
                raise ImageError("non-utf8-path", repr(name)) from None
            if "size" in pax:
                size = int(pax["size"])
            mtime = int(pax["mtime"].split(".")[0]) if "mtime" in pax else _parse_octal(block[136:148], "mtime")
            if kind != REG and size:
                raise ImageError("tar-header", f"{text}: non-file member with data")
            if text.endswith("/") and kind == DIR:
                text = text[:-1]
            self.pending = size + (-size % BLOCK)
            self.remaining = size
            return Member(text, kind, size, _parse_octal(block[100:108], "mode"), mtime, link, pax)

    def read_data(self, limit=None):
        if limit is not None and self.remaining > limit:
            raise ImageError("too-large", "member exceeds limit")
        data = self._read_exact(self.remaining)
        self._read_exact(self.pending - self.remaining)
        self.pending = self.remaining = 0
        return data

    def copy_data(self, out):
        while self.remaining:
            chunk = self._read_exact(min(self.remaining, 1 << 20))
            out.write(chunk)
            self.remaining -= len(chunk)
            self.pending -= len(chunk)
        self.skip_data()

    def skip_data(self):
        if self.pending:
            self._read_exact(self.pending)
        self.pending = self.remaining = 0
