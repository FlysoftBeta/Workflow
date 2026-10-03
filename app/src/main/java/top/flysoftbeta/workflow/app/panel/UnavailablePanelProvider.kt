package top.flysoftbeta.workflow.app.panel

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.material3.Text
import top.flysoftbeta.workflow.core.layout.Panel
import top.flysoftbeta.workflow.ui.design.icons.Sym

/** An unsupported panel reports a real error; it cannot fabricate service data or actions. */
internal class UnavailablePanelProvider : PanelProvider {
    override fun create(panel: Panel, context: PanelContext): PanelController = object : PanelController {
        override val tab = PanelTab("面板不可用", Sym.Info)
        @Composable override fun Content(frame: PanelFrame, modifier: Modifier) {
            Text("当前版本不支持此面板", modifier)
        }
    }
}
