"""metadata.base and the workspace fields of metadata.json, derived from versions.env and the
provisioned guest's `envctl state`. Shared by build.sh (podman) and guest/pack-in-guest.sh (engine)."""
import hashlib
import shlex

PLATFORMS = {"amd64": "linux/amd64", "arm64": "linux/arm64/v8"}
GUEST_PATH = ("/opt/toolchains/active/python/bin:/opt/toolchains/active/node/bin:"
              "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin")
# Guest directories that are persistent stores on the device; the image only seeds them (§2.3).
STORES = {"/home/work": {"store": "home/work", "seed": "if-absent"},
          "/opt/toolchains": {"store": "toolchains", "seed": "merge"}}


def read_versions(path):
    values = {}
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line and not line.startswith("#") and "=" in line:
                key, value = line.split("=", 1)
                values[key] = " ".join(shlex.split(value))
    return values


def base(versions, architecture):
    return {"reference": versions["DEBIAN_REFERENCE"], "indexDigest": versions["DEBIAN_INDEX_DIGEST"],
            "manifestDigest": versions["DEBIAN_DIGEST_" + architecture.upper()],
            "platform": PLATFORMS[architecture]}


def workspace(versions, state, provision_script, builder):
    with open(provision_script, "rb") as handle:
        provision = hashlib.sha256(handle.read()).hexdigest()
    if not state.get("python") or not state.get("node") or not state.get("profile"):
        raise ValueError("the provisioned guest reports no active toolchain profile")
    return {
        "user": {"name": "work", "uid": 1000, "gid": 1000, "home": "/home/work", "shell": "/bin/bash"},
        "environment": {"PATH": GUEST_PATH, "LANG": "en_US.UTF-8", "NVM_DIR": "/opt/toolchains/nvm",
                        "UV_PYTHON_INSTALL_DIR": "/opt/toolchains/uv/python", "UV_PYTHON_PREFERENCE": "system"},
        "stores": STORES,
        "toolchains": {"uv": versions["UV_VERSION"], "nvm": versions["NVM_VERSION"],
                       "python": state["python"], "node": state["node"], "profile": state["profile"]},
        "defaults": {"python": versions["DEFAULT_PYTHON"], "node": versions["DEFAULT_NODE"]},
        "provision": {"sha256": provision, "builder": builder},
    }
