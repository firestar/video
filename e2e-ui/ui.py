"""Playwright helpers for the real-input UI e2e suite.

"Real input" = the browser's own input pipeline:
  - text: page.type()  -> genuine CDP key events
  - clicks: page.click() -> genuine CDP mouse events
  - files: set_input_files -> the real <input type=file> change pipeline
  - swipe: CDP Input.dispatchTouchEvent (touchStart/touchMove/touchEnd)
  - wheel: CDP Input.dispatchMouseEvent type=mouseWheel

Screenshots land in .peckboard/ui-e2e/<run_id>/ for evidence.
"""
import os
import time

from playwright.sync_api import sync_playwright

BASE = os.environ.get("E2E_UI_BASE", "https://video.synload.com")
SHOT_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..",
                        ".peckboard", "ui-e2e")

DESKTOP_UA = (
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
)
PHONE_UA = (
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 "
    "(KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1"
)


class UI:
    """Wraps a Playwright browser + per-role contexts. Call ui.close() at the end."""

    def __init__(self, run_id):
        self.run_id = run_id
        self.shot_dir = os.path.abspath(os.path.join(SHOT_DIR, str(run_id)))
        os.makedirs(self.shot_dir, exist_ok=True)
        self._pw = sync_playwright().start()
        self.browser = self._pw.chromium.launch(headless=True)
        self.contexts = {}   # name -> BrowserContext
        self.pages = {}      # name -> Page

    # ---- contexts / login ----
    def new_context(self, name, mobile=False):
        if mobile:
            ctx = self.browser.new_context(
                viewport={"width": 375, "height": 812},
                has_touch=True, is_mobile=True, user_agent=PHONE_UA,
                device_scale_factor=2,
            )
        else:
            ctx = self.browser.new_context(
                viewport={"width": 1280, "height": 720},
                user_agent=DESKTOP_UA,
            )
        self.contexts[name] = ctx
        self.pages[name] = ctx.new_page()
        return self.pages[name]

    def ui_login(self, name, email, password, mobile=False):
        """Log in through the /login page with real typing. Returns the page."""
        page = self.new_context(name, mobile=mobile)
        page.goto(BASE + "/login", wait_until="domcontentloaded")
        page.get_by_placeholder("Email").type(email, delay=15)
        page.get_by_placeholder("Password").type(password, delay=15)
        page.get_by_role("button", name="Login", exact=True).click()
        page.wait_for_url(f"{BASE}/", timeout=30000)
        return page

    # ---- evidence ----
    def shot(self, page, label):
        safe = label.replace(" ", "_").replace("/", "-")
        path = os.path.join(self.shot_dir, f"{safe}.png")
        try:
            page.screenshot(path=path)
        except Exception:
            pass
        return path

    # ---- real input ----
    def cdp(self, page):
        return page.context.new_cdp_session(page)

    def real_swipe(self, page, x, y, dist, steps=14, hold_ms=40):
        """Genuine touch swipe: touchStart at (x,y), `steps` touchMove events
        moving up by `dist` px, touchEnd. Dispatched through CDP."""
        cdp = self.cdp(page)
        cdp.send("Input.dispatchTouchEvent",
                 {"type": "touchStart", "touchPoints": [{"x": x, "y": y}]})
        for i in range(1, steps + 1):
            cdp.send("Input.dispatchTouchEvent",
                     {"type": "touchMove",
                      "touchPoints": [{"x": x, "y": y - dist * i / steps}]})
            time.sleep(hold_ms / 1000)
        cdp.send("Input.dispatchTouchEvent", {"type": "touchEnd", "touchPoints": []})
        cdp.detach()

    def real_wheel(self, page, x, y, delta_y):
        """Genuine mouse wheel event through CDP (negative delta_y = scroll down)."""
        cdp = self.cdp(page)
        cdp.send("Input.dispatchMouseEvent",
                 {"type": "mouseWheel", "x": x, "y": y,
                  "deltaX": 0, "deltaY": delta_y})
        cdp.detach()

    def close(self):
        try:
            for ctx in self.contexts.values():
                try:
                    ctx.close()
                except Exception:
                    pass
            self.browser.close()
            self._pw.stop()
        except Exception:
            pass


# ---- shorts feed readers (shared by the swipe/wheel test) ----

def shorts_state(page):
    """Snapshot of the /shorts feed: scroll position + per-card video state."""
    return page.evaluate("""() => {
      const c = document.querySelector('[data-testid="shorts-feed"]');
      if (!c) return null;
      const cards = [...c.querySelectorAll('[data-testid="short-card"]')];
      const clientH = c.clientHeight;
      const scrollTop = c.scrollTop;
      return {
        scrollTop, clientH,
        activeByScroll: clientH ? Math.round(scrollTop / clientH) : 0,
        count: cards.length,
        per: cards.map((el, i) => {
          const v = el.querySelector('video');
          return {
            idx: i,
            title: (el.querySelector('[data-testid="short-title"]') || {}).textContent || '',
            hasVideo: !!v,
            paused: v ? v.paused : null,
            currentTime: v ? v.currentTime : null,
            readyState: v ? v.readyState : null,
          };
        }),
      };
    }""")


def wait_playing(page, idx, timeout=15):
    """Wait until card `idx` has a playing video. Returns its state or None."""
    t0 = time.time()
    while time.time() - t0 < timeout:
        st = shorts_state(page)
        if st and st["count"] > idx:
            p = st["per"][idx]
            if p["hasVideo"] and p["paused"] is False and p["readyState"] and p["readyState"] >= 2:
                return p
        time.sleep(0.5)
    return None
