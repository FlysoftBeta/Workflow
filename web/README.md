# Offline terminal assets

This npm package vendors xterm.js, fit/web-links addons and the Android 9 core-js compatibility layer
into `app/src/main/assets/web/vendor/`, with exact versions, licenses and SHA-256 manifest entries.
`package.json` and `package-lock.json` are the dependency inputs. `vendor.mjs` copies the required
files and transpiles JavaScript to Chromium 66. Treat its `vendor/` output as generated assets;
change the source inputs or vendoring script instead of editing copied bundles.

The app's terminal page, link integration and Android input adapter are maintained under
`app/src/main/assets/web/`. The two `*.test.mjs` files exercise those shipped adapters and the
vendored terminal in JSDOM. `node_modules/` is an ignored installation cache. Run these commands
from this directory:

```sh
npm ci --ignore-scripts
npm run build
npm test
```

Tests cover Android IME input and terminal links. Real Android WebView acceptance remains in
`app/src/androidTest/` through `OfflineRendererTest`; JSDOM cannot establish browser rendering or
device input acceptance. Chat uses native Compose/CommonMark/JLaTeXMath; its retired browser
assets, source and licenses are preserved under `docs/archive/web-chat/` and are not bundled by
this package. See the [testing guide](../docs/development/testing.md) for the device workflow.
