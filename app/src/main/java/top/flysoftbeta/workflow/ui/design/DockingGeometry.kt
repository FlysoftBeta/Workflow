package top.flysoftbeta.workflow.ui.design

/**
 * Region sizes (dp) of docs/ui.md §3.2. Side = file browser / conversation list; aux = conversation
 * panel (360) or file side panel (400).
 */
data class DockingSpec(
    val sideDefault: Float = 264f,
    val sideMin: Float = 200f,
    val sideMaxFraction: Float = 0.40f,
    val auxDefault: Float = 360f,
    val auxMin: Float = 300f,
    val auxMaxFraction: Float = 0.55f,
    val centerMin: Float = 360f,
    /** Seam between regions and to the window edges. */
    val gap: Float = 4f,
    /** Below this window width the layout degrades to one column (§3.5). */
    val narrowBelow: Float = 600f,
)

/** Result of [resolveDocking]; all widths in dp. Overlay widths apply when the region is open but not docked. */
data class DockingDecision(
    val sideDocked: Boolean,
    val auxDocked: Boolean,
    val sideWidth: Float,
    val auxWidth: Float,
    val centerWidth: Float,
    val sideOverlayWidth: Float,
    val auxOverlayWidth: Float,
    /** Narrow window: the side region is a modal drawer with scrim, aux a full-width overlay. */
    val narrow: Boolean,
)

/**
 * Space-based docking (no fixed breakpoints): the side region docks while the centre keeps ≥ centerMin;
 * the aux region docks only if, after the side region, the centre still keeps ≥ centerMin. Otherwise the
 * side region becomes a left drawer and aux slides in from the right at min(400, W − 48).
 * Widths are clamped to their min/max (max as a fraction of the window) first.
 */
fun resolveDocking(
    windowWidth: Float,
    sideOpen: Boolean,
    auxOpen: Boolean,
    sideWidth: Float,
    auxWidth: Float,
    spec: DockingSpec = DockingSpec(),
): DockingDecision {
    val side = sideWidth.coerceIn(spec.sideMin, maxOf(spec.sideMin, spec.sideMaxFraction * windowWidth))
    val aux = auxWidth.coerceIn(spec.auxMin, maxOf(spec.auxMin, spec.auxMaxFraction * windowWidth))
    val available = windowWidth - 2 * spec.gap
    val narrow = windowWidth < spec.narrowBelow
    val sideDocked = !narrow && sideOpen && available - (side + spec.gap) >= spec.centerMin
    val afterSide = available - if (sideDocked) side + spec.gap else 0f
    val auxDocked = !narrow && auxOpen && afterSide - (aux + spec.gap) >= spec.centerMin
    val center = afterSide - if (auxDocked) aux + spec.gap else 0f
    return DockingDecision(
        sideDocked = sideDocked,
        auxDocked = auxDocked,
        sideWidth = side,
        auxWidth = aux,
        centerWidth = center,
        sideOverlayWidth = if (narrow) minOf(320f, 0.85f * windowWidth) else side,
        auxOverlayWidth = if (narrow) windowWidth else minOf(400f, windowWidth - 48f),
        narrow = narrow,
    )
}
