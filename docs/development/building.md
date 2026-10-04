# Building Android packages

Run the commands in this guide from the repository root on a Linux x86_64 host. Workflow builds one APK per Android ABI because each package includes an environment image for that architecture. The ABI flavor and the Debug/Release build type are independent choices:

| Flavor | Android ABI | Environment image architecture | Debug task | Release task |
| --- | --- | --- | --- | --- |
| `arm64` | `arm64-v8a` | `arm64` | `:app:android:assembleArm64Debug` | `:app:android:assembleArm64Release` |
| `x86_64` | `x86_64` | `amd64` | `:app:android:assembleX86_64Debug` | `:app:android:assembleX86_64Release` |

Choose the ABI supported by the Android target. Either flavor can run on compatible physical hardware or an emulator; the flavor does not describe that distinction. Both retain application ID `top.flysoftbeta.workflow`, version 1.0.0/versionCode 10000, and minSdk 28. The standard `:app:android:assembleDebug` and `:app:android:assembleRelease` tasks aggregate both flavors and produce two APKs, each containing one ABI and its corresponding image.

## Prepare the host

Use JDK 25 to run Gradle, Rust 1.93.1 through rustup, Python 3.11 or newer, Git, Bash, and the usual Linux build utilities, including `flock`, `readelf`, and `sha256sum`. The Android build scripts select the NDK's `linux-x86_64` toolchain. Image construction additionally requires rootless Podman, zstd, curl, tar, and `dpkg-deb`; release-key creation requires OpenSSL. Node.js and npm are needed when regenerating or testing the offline terminal assets.

The checked-in wrapper selects Gradle 9.8.0, while the version catalog pins Android Gradle Plugin 9.4.1, NDK 30.0.15729638, and CMake 4.3.0. Install Android SDK Platform 37 and Build-Tools 37.0.0. These are repository versions; consult [dependencies](dependencies.md) and its linked manifests when changing them.

Set `JAVA_HOME` to the JDK 25 installation, add its `bin` directory to `PATH`, and set `ANDROID_HOME` to the SDK installation. Alternatively, record the SDK path as `sdk.dir=/absolute/path/to/Android/Sdk` in the ignored `local.properties`. After installing Android command-line tools, install the required SDK components:

```sh
"$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" \
  "platform-tools" "platforms;android-37" "build-tools;37.0.0" \
  "ndk;30.0.15729638" "cmake;4.3.0"
rustup toolchain install 1.93.1
rustup target add --toolchain 1.93.1 aarch64-linux-android x86_64-linux-android
export RUSTUP_TOOLCHAIN=1.93.1
```

Keep that Rust selection in the shell used for builds so both Cargo and the directly invoked loader compiler use it. The app compiles Java/Kotlin bytecode for Java 17; that target is separate from the JDK used to run Gradle. Do not commit local SDK paths, credentials, or signing keys.

## Prepare the customized images

A fresh clone does not contain the ignored environment archives or the executable-prebuilt cache. Gradle obtains missing Mihomo and Codex from pinned manifests. Mihomo is staged as an Android local executable, while Codex enters the Engine tools payload. Archive and member identities are verified. An invalid cached executable fails verification and must be inspected before removing it to permit a new download. First-time dependency resolution and image construction require network access.

Gradle expects customized `workspace` images to exist before APK assembly. Build both explicitly:

```sh
image/build.sh --profile workspace --arch amd64
image/build.sh --profile workspace --arch arm64
```

The required input pairs are `artifacts/image/amd64/image.tar.zst` with `image.json` and `artifacts/image/arm64/image.tar.zst` with `image.json`. Each command also writes package inventories, image state, checksums, and logs beside the archive. A single-flavor APK build needs the matching image pair; an aggregate Debug or Release build needs both.

The image builder coordinates its compression through the shared build lock. On an x86_64 host, the arm64 command uses the pinned build-only QEMU through a rootless Podman user namespace and a private mount namespace. This cross-build path requires Linux 6.7 or newer and does not configure global host binfmt. See [environment construction](../engine/environment.md#building-and-checking-images) for the image inputs, checks, and runtime contract.

APK assembly verifies the image profile, architecture, archive size, and SHA-256. It fails when the required pair is missing or invalid. Assembly does not provision the image, and a `base` image cannot replace the customized environment. Prepared task checkouts can receive independent image snapshots through `tools/workflow prepare NAME`, as described in the [multi-agent workflow](multi-agent.md).

## Engine tools payload

Chat is linked into the Server binary, so the payload carries no chat service or Java runtime. Android production depends on `:app:client`; instrumentation uses the process-launch fixture from `:app:client` test fixtures.

APK assembly invokes `engine/tools/package.py` for the selected architecture. It produces `tools.json` and `tools.zip` under generated assets at `assets/environment/tools/`. The payload combines pinned Codex and the retained upstream notices. Its catalog records per-file size, SHA-256 and executable mode, archive identity, architecture, fixed guest entry points and the pinned optional Claude release. Ignored downloads live beneath `third_party/.cache/engine`; a corrupt existing cache fails rather than being silently replaced.

For a standalone distribution, package the same pair:

```sh
python3 engine/tools/package.py --architecture amd64 \
  --output artifacts/engine-tools/amd64
```

Use `arm64` for the other architecture. Supply that output directory with Server `--tools`; the embedded bootstrap uses the pair carried in the APK. Missing or invalid mandatory payload members fail explicitly. Engine performs verification and binds tools into existing generations without replaying post-scripts. Claude remains a pinned optional Engine-managed runtime installation, requested through its tool API rather than an Android download script.

## Build Debug APKs

Use the shared build-lock wrapper for direct Gradle commands:

```sh
tools/with-build-lock.sh ./gradlew :app:android:assembleDebug
```

For only one ABI, use either command:

```sh
tools/with-build-lock.sh ./gradlew :app:android:assembleArm64Debug
tools/with-build-lock.sh ./gradlew :app:android:assembleX86_64Debug
```

Gradle automatically cross-compiles the Rust Workspace Server, runtime, and loader, builds the local proxy guardian and native test support as configured, stages verified Mihomo and notices, builds the guest chat service and Engine tools payload, and packages the selected environment image. Codex is a guest payload executable, not `libcodex.so` in Android JNI libraries. No separate manual native build is needed. Generated inputs stay under `app/android/build/`; Cargo and other build caches remain ignored. Shared native preparation currently builds both Android ABIs even when assembling a single flavor, so install both Rust targets.

The Debug outputs use standard Android debug signing:

| Flavor | APK path |
| --- | --- |
| `arm64` | `app/android/build/outputs/apk/arm64/debug/android-arm64-debug.apk` |
| `x86_64` | `app/android/build/outputs/apk/x86_64/debug/android-x86_64-debug.apk` |

The wrapper uses the primary checkout's `artifacts/.gradle.lock`, including from linked task checkouts. Cargo uses at most two jobs. Do not set `WORKFLOW_BUILD_LOCK_HELD=1` yourself; it is an internal signal passed after the lease is held. Commands such as `tools/workflow check` and `tools/build-release.sh` manage their own lease and should be invoked directly.

## Build and verify Release APKs

Release signing belongs to the coordinator checkout. Initialize its key explicitly once:

```sh
tools/init-release-key.sh
```

The script creates the ignored `artifacts/signing/Workflow-release.p12` only when absent and leaves an existing key intact. It uses a passwordless PKCS#12 store with alias `workflow`; protect the file as a private signing credential and retain the same key for subsequent app updates. A build never generates or replaces it. Task preparation does not copy or link signing material.

To assemble both signed Release APKs without creating a delivery bundle:

```sh
tools/with-build-lock.sh ./gradlew :app:android:assembleRelease
```

Use `:app:android:assembleArm64Release` or `:app:android:assembleX86_64Release` for one flavor. These write `app/android/build/outputs/apk/arm64/release/android-arm64-release.apk` and `app/android/build/outputs/apk/x86_64/release/android-x86_64-release.apk`, respectively. Direct Gradle builds can select another compatible PKCS#12 key with `-Pworkflow.keystore=/absolute/path/to/key.p12`; the alias and password expectations stay the same. The delivery helper expects the default local key path.

For the verified release delivery, run:

```sh
tools/build-release.sh
```

The helper acquires the shared build lease, builds both Release variants, and copies and verifies them before releasing it. It checks v1/v2/v3 signatures, version and minSdk, non-debuggable flags, the exact packaged ABI, native executables, the verified Engine tools payload, offline terminal assets, and the customized image metadata and digest. Successful delivery writes:

| Output under `artifacts/delivery/1.0.0/` | Purpose |
| --- | --- |
| `Workflow-1.0.0-arm64-v8a-release.apk` | Signed arm64-v8a package |
| `Workflow-1.0.0-x86_64-release.apk` | Signed x86_64 package |
| `SHA256SUMS`, `release.json` | APK digests, sizes, ABI, version, and image identity |
| `arm64-v8a-signature.txt`, `x86_64-signature.txt` | Signature verification results |
| `arm64-v8a-package.txt`, `x86_64-package.txt` | Android package inspection |
| `build.log` | Gradle build output |

These delivery paths are reused by later runs, so preserve a completed delivery and its matching evidence before rebuilding when retention matters. Debug and Release use the same application ID but normally different signing keys; an existing installation can only be updated by a package signed with the compatible key.

## Verify the intended behavior

APK assembly establishes that packaging succeeded. Use [testing and evidence](testing.md) for host checks, frozen instrumentation APK pairs, the shared emulator lease, and the Android acceptance matrix. The default `app-unit`, `lint`, and `android-apk` suites select `x86_64`. That is the default test target, independent of the names of the ABI flavors. Release delivery still needs a startup and connection smoke test on the intended target because Debug instrumentation does not exercise Release package flags.
