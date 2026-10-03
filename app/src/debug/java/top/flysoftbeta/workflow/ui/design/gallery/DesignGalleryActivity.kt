package top.flysoftbeta.workflow.ui.design.gallery

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import top.flysoftbeta.workflow.ui.design.theme.ThemeMode
import top.flysoftbeta.workflow.ui.design.theme.UiDensity

/** Pages of the debug design gallery. */
enum class GalleryPage(val title: String) {
    Chrome("Chrome"), Menus("Menus"), Content("Content"), Layout("Layout + DnD"), Tokens("Tokens")
}

/** Synthetic drag feedback states for screenshots (page Layout). */
enum class GalleryDnd { Zone, Sort, Area, Scrim, Rejected }

data class GalleryConfig(
    val page: GalleryPage = GalleryPage.Chrome,
    val theme: ThemeMode = ThemeMode.Light,
    val density: UiDensity = UiDensity.Compact,
    val dnd: GalleryDnd? = null,
) {
    companion object {
        fun fromIntent(intent: Intent?): GalleryConfig {
            fun extra(name: String) = intent?.getStringExtra(name)?.lowercase()
            val dnd = GalleryDnd.entries.firstOrNull { it.name.lowercase() == extra("dnd") }
            return GalleryConfig(
                page = GalleryPage.entries.firstOrNull { it.name.lowercase() == extra("page") } ?: if (dnd != null) GalleryPage.Layout else GalleryPage.Chrome,
                theme = if (extra("theme") == "dark") ThemeMode.Dark else ThemeMode.Light,
                density = if (extra("density") == "standard") UiDensity.Standard else UiDensity.Compact,
                dnd = dnd,
            )
        }
    }
}

/**
 * Debug-only showcase of every ui.design component in both densities and themes. Not linked from the
 * product UI. See app/src/debug/AndroidManifest.xml for the adb command and extras.
 */
class DesignGalleryActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val config = GalleryConfig.fromIntent(intent)
        setContent { DesignGallery(config) }
    }
}
