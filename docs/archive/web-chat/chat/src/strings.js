/* Every user-visible string of the transcript page (zh-CN). */
export const S = {
  running: '进行中',
  took: '用时',
  done: '已完成',
  stopped: '已停止',
  failed: '出错了',
  retry: '重试',
  thinking: '思考中…',
  thought: '已思考',
  plan: '计划',
  showMore: '显示更多',
  showLess: '收起',
  showAll: n => `显示全部 ${n} 行`,
  truncated: '输出已截断',
  exitCode: n => `退出码 ${n}`,
  editedFiles: n => `已编辑 ${n} 个文件`,
  moreFiles: n => `再显示 ${n} 个文件`,
  copy: '复制',
  copied: '已复制',
  copyCode: '复制代码',
  copyMarkdown: '复制为 Markdown',
  fork: '分叉',
  forkHere: '从这里分叉',
  more: '更多',
  openFile: '打开文件',
  image: '图片',
  attachment: '附件',
  unknown: '未识别的活动',
  subagent: '子代理',
  args: '参数',
  result: '结果',
  joiner: '，',
  group: {
    read: n => `已读取 ${n} 个文件`,
    list: n => `已列出 ${n} 个目录`,
    search: n => `搜索了 ${n} 次`,
    run: n => `运行了 ${n} 条命令`,
    files: n => `编辑了 ${n} 个文件`,
    tool: n => `调用了 ${n} 个工具`,
    agent: n => `${n} 个子代理`
  },
  /* Row text when the app sends no title: [running, finished] verb + object. */
  verb: {
    run: ['正在运行', '已运行'],
    read: ['正在读取', '已读取'],
    list: ['正在列出', '已列出'],
    search: ['正在搜索', '已搜索'],
    files: ['正在编辑', '已编辑'],
    tool: ['正在调用', '已调用']
  },
  itemStatus: { failed: '失败', declined: '已拒绝', incomplete: '未完成' },
  fileCount: n => `${n} 个文件`,
  announce: { completed: d => `已完成，用时 ${d}`, interrupted: '已停止', failed: '出错了' }
};

/** 24m 38s · 1m 12s · 3s · 1h 5m */
export function duration(ms) {
  const s = Math.max(0, Math.floor((Number(ms) || 0) / 1000));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h) return `${h}h ${m}m`;
  if (m) return `${m}m ${s % 60}s`;
  return `${s}s`;
}
