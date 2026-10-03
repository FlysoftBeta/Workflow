package top.flysoftbeta.workflow.ui.design

import android.graphics.Bitmap
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.onRoot
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.ui.design.gallery.DesignGallery
import top.flysoftbeta.workflow.ui.design.gallery.GalleryConfig
import top.flysoftbeta.workflow.ui.design.gallery.GalleryPage
import top.flysoftbeta.workflow.ui.design.theme.ThemeMode
import top.flysoftbeta.workflow.ui.design.theme.UiDensity

/**
 * Screenshot path for the design system (debug variant only: the gallery lives in src/debug).
 * Renders every gallery page in both themes and densities and writes PNGs to the app's external files
 * dir `screenshots/`; pull them with
 * `adb pull /sdcard/Android/data/top.flysoftbeta.workflow/files/screenshots artifacts/ui/androidTest/`.
 * Popups (menus) are separate windows and are not part of these captures; use the gallery + screencap.
 */
@RunWith(AndroidJUnit4::class)
class DesignGalleryScreenshotTest {
    @get:Rule val compose = createComposeRule()

    private val output: File by lazy {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        File(context.getExternalFilesDir(null), "screenshots").apply { mkdirs() }
    }

    private fun capture(name: String) {
        compose.waitForIdle()
        val bitmap = compose.onRoot().captureToImage().asAndroidBitmap()
        val file = File(output, "$name.png")
        file.outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
        assertTrue("empty capture $name", bitmap.width > 0 && bitmap.height > 0)
    }

    @Test fun allPagesBothThemesBothDensities() {
        var config by mutableStateOf(GalleryConfig())
        compose.setContent {
            key(config) { DesignGallery(config) }
        }
        for (theme in listOf(ThemeMode.Light, ThemeMode.Dark)) {
            for (density in UiDensity.entries) {
                for (page in GalleryPage.entries.filter { it != GalleryPage.Menus }) {
                    compose.runOnIdle { config = GalleryConfig(page, theme, density) }
                    compose.onNodeWithText(page.title).assertExists()
                    capture("${page.name.lowercase()}-${theme.name.lowercase()}-${density.name.lowercase()}")
                }
            }
        }
    }
}
