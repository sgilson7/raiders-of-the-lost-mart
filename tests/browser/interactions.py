"""The project's own interactions for tests/browser/check.py.

A person's first minutes: the title, the workshop (put a part on the gun and
see the stats change), the client board, a solo run (out of the van and into
the lot), the menu, abandoning the run, and the results screen. Selectors and
waits only: Firefox's CSP blocks Playwright's string-form wait_for_function.
"""


def run(page, check, engine):
    page.wait_for_selector("body[data-mode='title']", state="attached", timeout=10000)
    check(page.is_visible("#title"), "the title screen shows")
    check(page.locator("#title img.backdrop").evaluate("i => i.complete && i.naturalWidth > 0"), "the TikZ title art loaded")

    page.click("#t-workshop")
    page.wait_for_selector("body[data-mode='workshop']", state="attached", timeout=5000)
    before = page.inner_text("#ws-stats")
    page.click("#ws-cats .part:has(img[src*='parts/smg.png'])")
    page.wait_for_timeout(300)
    after = page.inner_text("#ws-stats")
    check(before != after, "putting the SMG frame on changes the gun's numbers")
    icons = page.locator("#ws-cats img").count()
    check(icons == 30, f"thirty part icons on the shelves ({icons})")

    page.click("#ws-board")
    page.wait_for_selector("body[data-mode='board']", state="attached", timeout=5000)
    cards = page.locator("#b-list .mission").count()
    check(cards == 5, f"five client orders ({cards})")
    page.click("#b-list .mission:nth-child(5)")
    page.wait_for_timeout(200)
    check("Lights" in page.inner_text("#b-name"), "picking an order shows its brief")

    page.click("#b-go")
    page.wait_for_selector("body[data-mode='play']", state="attached", timeout=5000)
    page.wait_for_timeout(600)
    check("doors" in page.inner_text("#prompt-text").lower(), "the run starts in the van with the doors shut")
    page.keyboard.down("KeyE")
    page.wait_for_timeout(1200)
    page.keyboard.up("KeyE")
    page.keyboard.down("KeyW")
    page.wait_for_timeout(2500)
    page.keyboard.up("KeyW")
    zone = page.inner_text("#o-zone")
    check(zone != "", f"out in the world, the HUD names where ({zone})")
    check(page.locator("#markers .marker").count() >= 1, "markers show the order and the van")

    page.keyboard.press("Escape")
    page.wait_for_selector("body[data-mode='paused']", state="attached", timeout=5000)
    check(page.is_visible("#paused"), "Escape pauses a solo run")
    page.click("#quit")
    page.wait_for_selector("body[data-mode='results']", state="attached", timeout=5000)
    check(page.is_visible("#results"), "abandoning ends in the results")
    page.click("#r-next")
    page.wait_for_selector("body[data-mode='title']", state="attached", timeout=5000)
    check(True, "and back to the title")
