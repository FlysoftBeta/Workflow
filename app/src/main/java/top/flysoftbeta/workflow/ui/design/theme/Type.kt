package top.flysoftbeta.workflow.ui.design.theme

import android.content.res.AssetManager
import androidx.compose.material3.Typography
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

/**
 * Text styles of docs/ui.md §1.2. UI text uses the system font (Roboto / Noto Sans CJK);
 * [mono] uses the bundled JetBrains Mono NL (assets/fonts, shared with sora-editor and xterm).
 */
@Immutable
data class WorkflowTextStyles(
    /** 16/22 500: dialogs, sheets. */
    val titleMd: TextStyle,
    /** 14/20 500: header titles, session names. */
    val titleSm: TextStyle,
    /** 14/20: lists, settings, menus. */
    val body: TextStyle,
    /** 13/18: tabs, tree, activity rows. Use [labelActive] for the active/opened state. */
    val label: TextStyle,
    val labelActive: TextStyle,
    /** 12/16: time, secondary info, tile labels. */
    val caption: TextStyle,
    /** 11/14: ticks, badges. */
    val micro: TextStyle,
    /** 15/24: message body. */
    val chat: TextStyle,
    val chatH1: TextStyle,
    val chatH2: TextStyle,
    val chatH3: TextStyle,
    /** 13/19: editor, terminal. */
    val mono: TextStyle,
    /** 13/20: code blocks. */
    val monoBlock: TextStyle,
)

/** Asset paths of the bundled monospace font (also used by the editor and terminal adapters). */
object MonoFontAssets {
    const val REGULAR = "fonts/JetBrainsMonoNL-Regular.ttf"
    const val BOLD = "fonts/JetBrainsMonoNL-Bold.ttf"
}

fun monoFontFamily(assets: AssetManager?): FontFamily = if (assets == null) FontFamily.Monospace else FontFamily(
    Font(MonoFontAssets.REGULAR, assets, FontWeight.Normal),
    Font(MonoFontAssets.BOLD, assets, FontWeight.Bold),
)

fun workflowTextStyles(mono: FontFamily): WorkflowTextStyles {
    val medium = FontWeight.Medium
    return WorkflowTextStyles(
        titleMd = TextStyle(fontSize = 16.sp, lineHeight = 22.sp, fontWeight = medium),
        titleSm = TextStyle(fontSize = 14.sp, lineHeight = 20.sp, fontWeight = medium),
        body = TextStyle(fontSize = 14.sp, lineHeight = 20.sp),
        label = TextStyle(fontSize = 13.sp, lineHeight = 18.sp),
        labelActive = TextStyle(fontSize = 13.sp, lineHeight = 18.sp, fontWeight = medium),
        caption = TextStyle(fontSize = 12.sp, lineHeight = 16.sp),
        micro = TextStyle(fontSize = 11.sp, lineHeight = 14.sp),
        chat = TextStyle(fontSize = 15.sp, lineHeight = 24.sp),
        chatH1 = TextStyle(fontSize = 20.sp, lineHeight = 28.sp, fontWeight = FontWeight.SemiBold),
        chatH2 = TextStyle(fontSize = 18.sp, lineHeight = 26.sp, fontWeight = FontWeight.SemiBold),
        chatH3 = TextStyle(fontSize = 16.sp, lineHeight = 24.sp, fontWeight = FontWeight.SemiBold),
        mono = TextStyle(fontFamily = mono, fontSize = 13.sp, lineHeight = 19.sp),
        monoBlock = TextStyle(fontFamily = mono, fontSize = 13.sp, lineHeight = 20.sp),
    )
}

/**
 * M3 typography mapped onto the ui.md scale so stock M3 components (buttons, dialogs, text fields)
 * come out at the same compact sizes.
 */
fun workflowTypography(styles: WorkflowTextStyles): Typography = Typography(
    displayLarge = TextStyle(fontSize = 32.sp, lineHeight = 40.sp),
    displayMedium = TextStyle(fontSize = 28.sp, lineHeight = 36.sp),
    displaySmall = TextStyle(fontSize = 24.sp, lineHeight = 32.sp),
    headlineLarge = TextStyle(fontSize = 22.sp, lineHeight = 28.sp, fontWeight = FontWeight.Medium),
    headlineMedium = TextStyle(fontSize = 20.sp, lineHeight = 26.sp, fontWeight = FontWeight.Medium),
    // AlertDialog titles use headlineSmall; ui.md wants titleMd for dialogs.
    headlineSmall = styles.titleMd,
    titleLarge = TextStyle(fontSize = 18.sp, lineHeight = 24.sp, fontWeight = FontWeight.Medium),
    titleMedium = styles.titleMd,
    titleSmall = styles.titleSm,
    bodyLarge = styles.body,
    bodyMedium = styles.body,
    bodySmall = styles.caption,
    labelLarge = styles.labelActive,
    labelMedium = styles.caption.copy(fontWeight = FontWeight.Medium),
    labelSmall = styles.micro.copy(fontWeight = FontWeight.Medium),
)

val LocalWorkflowTextStyles = staticCompositionLocalOf { workflowTextStyles(FontFamily.Monospace) }
