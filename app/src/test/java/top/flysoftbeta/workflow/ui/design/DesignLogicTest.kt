package top.flysoftbeta.workflow.ui.design

import java.time.LocalDate
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ModelSliderGeometryTest {
    @Test fun detentsHaveEqualWidthAndSegmentsGaps() {
        // 3 + 5 efforts, 6px gap, 406px → unit 50.
        val layout = ModelSliderGeometry.layout(406f, 6f, listOf(3, 5))
        assertEquals(2, layout.segments.size)
        assertEquals(0f, layout.segments[0].start, 0.001f)
        assertEquals(150f, layout.segments[0].end, 0.001f)
        assertEquals(156f, layout.segments[1].start, 0.001f)
        assertEquals(406f, layout.segments[1].end, 0.001f)
        assertEquals(listOf(25f, 75f, 125f, 181f, 231f, 281f, 331f, 381f), layout.detents.map { it.x })
        assertEquals(4, layout.indexOf(model = 1, effort = 1))
    }

    @Test fun nearestDetentSnaps() {
        val layout = ModelSliderGeometry.layout(406f, 6f, listOf(3, 5))
        assertEquals(0, ModelSliderGeometry.nearest(layout, -100f))
        assertEquals(2, ModelSliderGeometry.nearest(layout, 150f)) // 125 is nearer than 181
        assertEquals(3, ModelSliderGeometry.nearest(layout, 160f))
        assertEquals(7, ModelSliderGeometry.nearest(layout, 1000f))
        assertEquals(-1, ModelSliderGeometry.nearest(ModelSliderGeometry.layout(0f, 6f, listOf(3)), 1f))
    }
}

class DockingGeometryTest {
    // Expected values from the docs/ui.md §3.2 table.
    @Test fun landscape261AllDocked() {
        val files = resolveDocking(1177f, sideOpen = true, auxOpen = true, sideWidth = 264f, auxWidth = 360f)
        assertTrue(files.sideDocked); assertTrue(files.auxDocked)
        assertEquals(537f, files.centerWidth, 0.5f)
        val chat = resolveDocking(1177f, true, true, 264f, 400f)
        assertTrue(chat.auxDocked)
        assertEquals(497f, chat.centerWidth, 0.5f)
    }

    @Test fun landscape320AuxBecomesOverlayUntilSideCollapses() {
        val d = resolveDocking(960f, true, true, 264f, 360f)
        assertTrue(d.sideDocked); assertFalse(d.auxDocked)
        assertEquals(684f, d.centerWidth, 0.5f)
        assertEquals(400f, d.auxOverlayWidth, 0f)
        val collapsed = resolveDocking(960f, false, true, 264f, 360f)
        assertTrue(collapsed.auxDocked)
    }

    @Test fun portraitAndNarrow() {
        val portrait = resolveDocking(736f, true, true, 264f, 360f)
        assertTrue(portrait.sideDocked); assertFalse(portrait.auxDocked)
        assertEquals(460f, portrait.centerWidth, 0.5f)
        val narrow = resolveDocking(411f, true, true, 264f, 360f)
        assertFalse(narrow.sideDocked); assertFalse(narrow.auxDocked); assertTrue(narrow.narrow)
        assertEquals(320f, narrow.sideOverlayWidth, 0.5f)
        assertEquals(411f, narrow.auxOverlayWidth, 0f)
    }

    @Test fun widthsClampToMinAndMax() {
        val d = resolveDocking(1000f, true, true, 50f, 900f)
        assertEquals(200f, d.sideWidth, 0f)
        assertEquals(550f, d.auxWidth, 0f)
    }
}

class ExtraKeysStateTest {
    @Test fun tapIsOneShotDoubleTapLocks() {
        val state = ExtraKeysState()
        state.tap(ModifierKey.Ctrl, 1000)
        assertEquals(Latch.OneShot, state.ctrl)
        assertEquals(KeyModifiers(ctrl = true), state.consume())
        assertEquals(Latch.Off, state.ctrl)

        state.tap(ModifierKey.Ctrl, 2000)
        state.tap(ModifierKey.Ctrl, 2200)
        assertEquals(Latch.Locked, state.ctrl)
        assertEquals(KeyModifiers(ctrl = true), state.consume())
        assertEquals(Latch.Locked, state.ctrl) // locked survives key presses
        state.tap(ModifierKey.Ctrl, 3000)
        assertEquals(Latch.Off, state.ctrl)
    }

    @Test fun slowSecondTapReleasesAndModifiersAreIndependent() {
        val state = ExtraKeysState()
        state.tap(ModifierKey.Alt, 0)
        state.tap(ModifierKey.Alt, 1000)
        assertEquals(Latch.Off, state.alt)
        state.tap(ModifierKey.Ctrl, 2000)
        state.tap(ModifierKey.Alt, 2100)
        assertEquals(Latch.OneShot, state.ctrl)
        assertEquals(Latch.OneShot, state.alt) // a tap on another key is not a double tap
        assertEquals(KeyModifiers(ctrl = true, alt = true), state.consume())
        assertTrue(state.consume().none)
    }
}

class TimelineAgeTest {
    private val today = LocalDate.of(2026, 9, 27) // a Sunday

    @Test fun buckets() {
        assertEquals(TimelineAge.Today, TimelineAge.of(today, today))
        assertEquals(TimelineAge.Yesterday, TimelineAge.of(today.minusDays(1), today))
        assertEquals(TimelineAge.ThisWeek, TimelineAge.of(LocalDate.of(2026, 9, 21), today)) // Monday
        assertEquals(TimelineAge.Older, TimelineAge.of(LocalDate.of(2026, 9, 20), today))
        // On a Monday, Sunday is "yesterday", Saturday is older.
        val monday = LocalDate.of(2026, 9, 28)
        assertEquals(TimelineAge.Yesterday, TimelineAge.of(today, monday))
        assertEquals(TimelineAge.Older, TimelineAge.of(today.minusDays(1), monday))
    }

    @Test fun fadingIsMonotonicWithContrastFloor() {
        val alphas = TimelineAge.entries.map { it.alpha }
        assertEquals(alphas.sortedDescending(), alphas)
        assertTrue(alphas.min() >= 0.62f)
        assertEquals(listOf(3, 2, 1, 1), TimelineAge.entries.map { it.maxResources })
    }
}
