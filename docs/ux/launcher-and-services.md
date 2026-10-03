# Launcher and service panels

Launcher and service panels use the same [design system](design-system.md) as Workbench. Their actions follow the [product specification](../product/workspace-and-services.md): controls report actual state, and unavailable capabilities expose a useful authorization action.

## Launcher and Apps

Launcher uses a `surface` background with a 40dp corner row and no title, slogan, or Continue working card. The Session control has a transparent background and a 48dp touch target. Its grid is top-aligned and horizontally centered. Tiles are 88×92dp with a 56dp icon and caption. Columns use a nominal 104dp pitch, with at most ten columns and a maximum grid width of 976dp. Available width determines the actual column count.

Built-in glyphs sit on primary-container tiles. A name collision with a third-party app adds the Workflow qualifier, such as Workflow Settings. Pending agent decisions add an 8dp dot to Workbench. Third-party app icons display as supplied by the installed app.

The long-press menu is anchored to the tile and includes icons. It offers Open, then Open in separate Session for Proxy or Settings, or New Session for Workbench. Third-party entries also offer App info and Remove from Apps. Built-in Apps remain reorderable but cannot be removed. After a long press, movement beyond touch slop starts reordering: the dragged tile scales to 1.08 with a subtle shadow around the icon only and other tiles spring aside.

Add app opens a 560dp-wide sheet with a maximum height of 80% at window widths of at least 600dp; narrower windows use full screen. A 40dp search field sits above 44dp rows containing a 32dp icon, app name, and checkbox. Changes apply immediately, and Done is at the top right. Package names are absent. Only duplicate names receive a caption identifying their source.

## The floating control

Collapsed, the control is a 44dp circle with a 48dp touch target, a 24dp app glyph, and highest-container fill. It rests half-hidden against the screen edge and fades to 0.45 opacity after three idle seconds. Dragging restores full opacity and scales it to 1.1. Release springs it to the closest left or right edge and keeps it within the vertical safe area. Its remembered side and proportional vertical position survive rotation. Pending requests add an 8dp dot; the control hides on the lock screen.

The expanded panel appears on the button's side, vertically centered around it and clamped to the screen. It is 320dp wide, with high-container fill, `xl` corners, and 12dp padding, and opens by scaling and fading from the button. Tapping outside, Back, or Home closes it.

Apps occupy four columns of 68×72dp cells, each with a 44dp icon and micro label. Apps from Launcher appear first in Launcher order, followed by extra apps selected only for the floating panel. The grid shows at most three rows before scrolling. Its final Edit cell opens Settings → Floating control → Extra apps. Launching an app closes the panel.

A 40dp Expressive brightness slider embeds a sun icon in its track and applies changes while dragging. An optional automatic-brightness control sits alongside it. Grayscale and Lock screen share the next row as equal-width 56dp buttons. The grayscale toggle changes shape when selected and reflects the system's actual grayscale state. Lock screen closes the panel before locking.

If a capability is unavailable, its control is disabled with an adjacent Authorize action leading directly to the relevant permission route. Readback determines displayed state. A failed operation displays a local error caption for two seconds.

## Proxy

The panel title is Proxy, without the underlying engine's name. Its 48dp status strip holds a morphing start/stop control, the Rule/Global/Direct mode selector, and live upload/download rates. Without root, the control becomes Request root. Proxy groups have 40dp headers; nodes use adaptive columns of 36dp chips. Manual and automatic-test group types are distinguished by icons rather than explanatory labels.

Configuration still supplies the displayed modes and groups while stopped, but their controls are disabled without an extra explanation below them. A selected node uses secondary-container fill and a checkmark. Latency below 300ms uses success color, latency from 300ms to below 800ms uses warning, and higher latency or timeout uses error.

Another active VPN or TUN produces a one-line error-container banner reading Another VPN is running with a View action. Logs and Connections open as separate Panels in the same Stack and add no title bars of their own. Without configuration, the content is simply No proxy configuration with Import configuration and Edit configuration.

## Settings

At panel widths of at least 720dp, Settings has a 220dp category column and a detail column. Narrower panels navigate into categories, with Back inside the existing header. Rows are 48dp high, with a body title and either a caption current value or a switch on the right. The page has no explanatory subtitles, exposed file paths, taglines, or Termux/pairing controls.

| Category | Content |
| --- | --- |
| Appearance | Theme, density, editor and terminal font sizes |
| Accounts | One row each for Codex and Claude Code; account name and Log out when signed in, or Log in when signed out. Login opens a sheet with browser or device-code choices. |
| Environment | Ready, Building n%, or Restart needed. Restart environment appears when a verified pending environment exists. |
| Floating control | Enablement reflecting actual permission, extra apps, reset position |
| Default Home | Current status and Set as default |
| Permissions | Floating control, modify system settings, grayscale, screen locking, root, and notifications; each shows measured state and Authorize when needed |
| About | Version and open-source licenses |
