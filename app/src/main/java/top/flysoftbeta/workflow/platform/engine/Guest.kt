package top.flysoftbeta.workflow.platform.engine

object Guest {
    const val WORKSPACE = "/workspace"
    const val INTERNAL = "/workspace/.workspace"
    const val HOME = "/home/work"
    const val TOOLCHAINS = "/opt/toolchains"
    const val RESOLV_CONF = "/etc/resolv.conf"
    const val CODEX = "/opt/workflow/bundled/libcodex.so"
    const val USER = "work"
    const val SHELL = "/bin/bash"
    const val CODEX_HOME = "$HOME/.codex"
    const val CLAUDE_CONFIG = "$HOME/.claude"
}
