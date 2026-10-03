/* The document scrolls. Follow mode = pinned to the bottom: content growth jumps (no smooth scroll);
 * any user scroll up leaves follow mode, reaching the bottom re-enters it. Reports layout{atBottom,height}
 * at most once per frame and only on change; asks for earlier history near the top. */
import { post } from './bridge.js';
import { onFrameEnd, requestFrame } from './frame.js';

const EARLIER_PX = 240;
const BOTTOM_SLOP = 4;

export class Scroller {
  constructor() {
    this.follow = true;
    this.expectTop = -1;
    this.reported = null;
    this.hasEarlier = false;
    this.earlierPending = false;
    this.touchY = null;
    onFrameEnd(() => this.frameEnd());
    window.addEventListener('scroll', () => this.onScroll(), { passive: true });
    window.addEventListener('resize', () => requestFrame(), { passive: true });
    window.addEventListener('wheel', event => { if (event.deltaY < 0) this.leave(); }, { passive: true });
    window.addEventListener('touchstart', event => { this.touchY = event.touches[0] ? event.touches[0].clientY : null; }, { passive: true });
    window.addEventListener('touchmove', event => {
      const t = event.touches[0];
      if (t && this.touchY != null && t.clientY - this.touchY > 6) this.leave();
      if (t) this.touchY = t.clientY;
    }, { passive: true });
    if (typeof ResizeObserver === 'function') new ResizeObserver(() => requestFrame()).observe(document.documentElement);
    document.addEventListener('load', () => requestFrame(), true); // images changing height
  }

  get root() { return document.scrollingElement || document.documentElement; }

  metrics() {
    const root = this.root;
    const view = window.innerHeight || root.clientHeight;
    return { top: root.scrollTop, height: root.scrollHeight, bottom: Math.max(0, root.scrollHeight - view) };
  }

  jump() {
    const root = this.root;
    root.scrollTop = this.metrics().bottom;
    this.expectTop = root.scrollTop;
  }

  leave() {
    if (!this.follow) return;
    this.follow = false;
    requestFrame();
  }

  onScroll() {
    const m = this.metrics();
    if (Math.abs(m.top - this.expectTop) > 1) {
      this.expectTop = -1;
      this.follow = m.top >= m.bottom - BOTTOM_SLOP;
    }
    requestFrame();
  }

  frameEnd() {
    if (this.follow) this.jump();
    const m = this.metrics();
    const r = this.reported;
    if (!r || r.atBottom !== this.follow || r.height !== m.height) {
      this.reported = { atBottom: this.follow, height: m.height };
      post({ type: 'layout', atBottom: this.follow, height: m.height });
    }
    if (this.hasEarlier && !this.earlierPending && m.top < EARLIER_PX) {
      this.earlierPending = true;
      post({ type: 'earlier' });
    }
  }

  toBottom() {
    this.follow = true;
    this.jump();
    requestFrame();
  }

  setFollow(follow) {
    if (follow) this.toBottom();
    else this.leave();
  }

  setEarlier(hasEarlier) {
    this.hasEarlier = !!hasEarlier;
    this.earlierPending = false;
  }

  /** Run a mutation that only adds content above the viewport, keeping the visible content in place. */
  keepAnchor(mutate) {
    const root = this.root;
    const before = root.scrollHeight;
    const top = root.scrollTop;
    mutate();
    if (this.follow) return;
    root.scrollTop = top + (root.scrollHeight - before);
    this.expectTop = root.scrollTop;
  }
}
