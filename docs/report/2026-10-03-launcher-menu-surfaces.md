# Launcher and menu surface corrections

The screenshot review identified a rectangular shadow around a dragged Launcher tile, a filled Session control occupying the full Launcher touch height, and menu shadows cut off at the bottom. Permission descriptions also shared one ellipsized line with their titles.

Launcher now elevates only the icon, leaving its caption without a rectangular shadow and preserving the attention badge. The shared Session control has a transparent resting background while retaining its touch target and clipped ripple. Composite menus reserve 12dp inside their scroll viewport for shadows; list menus and model popovers reserve the same space inside their popup bounds. Nested menus inherit the composite correction. Permission choices render their descriptions separately and allow wrapping. Approval behavior is unchanged.

The audit also inspected the docking and workbench overlays, Session sheet, generic drag preview, and terminal selection toolbar. These do not put elevated menu surfaces directly against a scrolling viewport boundary, so no matching correction was applied there. Pre-existing tab geometry and density edits in the working checkout were retained.

## Verification

The final `android-apk` run is `20261003T073544Z-70d1be48`, with source fingerprint `f6317e1291c8d456a35027d2c45fa2b77fd509389dca7004322410c583228e63`. App APK SHA-256 is `509282a8ecf87c55a5a9d98ab20c79b368e01eb18555e8cb0a82632586865b78`; instrumentation APK SHA-256 is `158d654606de80907f6843575739e04e01076e4f03cc555ddd5fd333dc9f9a32`.

Device run `20261003T073614Z-76b9eed1` passed both methods of `MenuSurfaceRegressionTest`, with zero skips, on the isolated `workflow-api28` AVD: Android 9/API 28, x86_64, 1080×1920 at 420 dpi. The tests exercise drag reordering and readable permission descriptions, including complete text, absence of ellipsis, and pixel bounds with a one-pixel rounding allowance. Full-window captures cover the dragged icon and both menu examples in light and dark themes. Screenshots are retained under `artifacts/ui/menu-regression-settled/menu-regression/`.

Earlier failed runs remain in the run archive. They exposed test-harness issues: pointer event timestamps alone did not advance the long-press timeout, merged semantics included title layouts, and integer text-width rounding made `hasVisualOverflow` too strict. Each theme/menu sample now creates a fresh popup to avoid capturing an intermediate window position after a size change. The final captures were inspected for icon-only elevation and complete menu edges.

These are component rendering checks without a backend or credentials. They do not claim full application acceptance on physical ARM64 or newer Android devices. The user's daily device was not modified. This report was added after the source-bound build and device checks; runtime source and test source were unchanged afterward.
