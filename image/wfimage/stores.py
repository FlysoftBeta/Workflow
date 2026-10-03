"""metadata.stores: guest directories whose image content only seeds a persistent store (§2.3, §2.4).

Rows strictly below a store prefix are extracted into seeds/<store>/ instead of the generation. They
must be plain files, directories or symlinks owned by the default user without set-id/sticky bits,
because stores have no attribute store: the guest sees host permission bits and the default owner."""
import re

from .common import SEED_POLICIES, ImageError, check_guest_path, encode_path

STORE_NAME = re.compile(r"^[a-z0-9][a-z0-9._-]*(/[a-z0-9][a-z0-9._-]*)*$")


def parse(metadata):
    """{guest prefix bytes: (store, policy)} from metadata.stores (absent for base images)."""
    stores = metadata.get("stores", {})
    if not isinstance(stores, dict):
        raise ImageError("metadata", "stores")
    parsed = {}
    for prefix, value in stores.items():
        if not isinstance(value, dict) or not isinstance(value.get("store"), str) \
                or not STORE_NAME.match(value["store"]) or value.get("seed") not in SEED_POLICIES:
            raise ImageError("metadata", f"stores[{prefix}]")
        raw = check_guest_path(prefix.encode("utf-8"))
        if raw == b"/":
            raise ImageError("metadata", "the root cannot be a store")
        parsed[raw] = (value["store"], value["seed"])
    prefixes = sorted(parsed)
    for index, prefix in enumerate(prefixes):
        if any(other.startswith(prefix + b"/") for other in prefixes[index + 1:]):
            raise ImageError("metadata", "nested stores")
    if len({store for store, _ in parsed.values()}) != len(parsed):
        raise ImageError("metadata", "duplicate store names")
    return parsed


def owner_of(path, stores):
    """(prefix, store, relative bytes) when path lies strictly below a store prefix, else None."""
    for prefix, (store, _) in stores.items():
        if path.startswith(prefix + b"/"):
            return prefix, store, path[len(prefix) + 1:]
    return None


def check_rows(rows, stores, user):
    """Store prefixes must be directories; rows below them must be representable as plain files."""
    if not stores:
        return 0
    kinds = {row.path: row.type for row in rows}
    for prefix in stores:
        if kinds.get(prefix) != "d":
            raise ImageError("store-seed", "store prefix is not a directory: " + encode_path(prefix))
    seeds = 0
    for row in rows:
        if row.type == "h" and owner_of(row.target, stores) is not None:
            raise ImageError("store-seed", f"{encode_path(row.path)}: hardlink into a store")
        if owner_of(row.path, stores) is None:
            continue
        seeds += 1
        if row.type not in "dfl":
            raise ImageError("store-seed", f"{encode_path(row.path)}: type {row.type} cannot be seeded")
        if user is None or (row.uid, row.gid) != user:
            raise ImageError("store-seed", f"{encode_path(row.path)}: not owned by the default user")
        if row.mode & 0o7000:
            raise ImageError("store-seed", f"{encode_path(row.path)}: set-id or sticky bit")
    return seeds
