# Offline terminal assets

This npm package vendors xterm.js, fit/web-links addons and the Android 9 core-js compatibility layer
into `app/src/main/assets/web/vendor/`, with exact versions, licenses and SHA-256 manifest entries.
`vendor.mjs` transpiles JavaScript to Chromium 66. The app's terminal adapter and page are maintained
under `app/src/main/assets/web/`.

```sh
npm ci --ignore-scripts
npm run build
npm test
```

Tests cover Android IME input and terminal links. Real Android WebView acceptance remains in
`OfflineRendererTest`. Chat uses native Compose/CommonMark/JLaTeXMath; its retired browser assets,
source and licenses are preserved under `docs/archive/web-chat/` and are not bundled by this package.
