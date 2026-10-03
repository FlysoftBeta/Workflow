# User experience specification

Workflow's interface keeps the current work visible, fits a tablet's limited height, and works with touch and a soft keyboard. The [product specification](../product/README.md) defines behavior; this directory defines how that behavior looks and feels. Translating this specification does not change application localization or shipped text.

Start with the [design system](design-system.md) for density, typography, color, motion, accessibility, and shared feedback. Use [Workbench interactions](workbench.md) when designing layouts, tabs, file editing, terminals, Session navigation, or dragging. [Conversations](conversations.md) covers message streams, activity, approval cards, attachments, and model selection. [Launcher and services](launcher-and-services.md) covers Apps, floating controls, Proxy, and Settings.

These specifications target Android 9/API 28 as well as newer devices. The layout must remain usable on 1920×1080 and 1920×1200 displays, at smaller scaled widths, and with the keyboard open. [Engine](../engine/README.md) and [App](../app/README.md) documentation own component technology and state persistence. [Development documentation](../development/README.md) owns verification and the distinction between intended behavior and demonstrated acceptance.
