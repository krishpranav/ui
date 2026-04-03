# E2E Tests — Playwright + Leptos SSR

Gotchas and fixes required to make Playwright work correctly with Leptos SSR + WASM hydration.

---

## 1. Wait for WASM hydration before interacting

**Problem:** Leptos apps are server-side rendered first, then hydrated by a WASM bundle.
Playwright's `goto()` resolves when the HTML is loaded, but the WASM may not have finished
attaching event handlers yet. Clicking elements before hydration succeeds silently — the
element is visible and enabled, but the `on:click` handler is not registered yet.

**Symptom:** Click actions appear to succeed (no timeout), but reactive state never updates.

**Fix:** Override `goto()` in your page object and wait for `networkidle`:

```typescript
override async goto(section?: string) {
  await super.goto(section);
  await this.page.waitForLoadState("networkidle");
}
```

`networkidle` waits for all network activity to settle, which includes the WASM bundle
download and initialization. After this, Leptos event handlers are attached.

---

## 2. Use Playwright auto-retry assertions for reactive attributes

**Problem:** Leptos reactive attributes update asynchronously after an event fires.
Reading an attribute immediately after a click with `getAttribute()` may return the
stale SSR value — the reactive effect hasn't run yet.

**Symptom:** `expect(await locator.getAttribute("data-active")).toBe("true")` fails
intermittently or consistently, even though the DOM does eventually update.

**Fix:** Use `toBeVisible()` or `toHaveAttribute()` instead, which retry automatically
until the condition is met (up to the configured timeout):

```typescript
// Bad — reads once, no retry
expect(await page.locator('[data-active="true"]').getAttribute("data-color")).toBe("red");

// Good — retries until visible
await expect(page.locator('[data-color="red"][data-active="true"]')).toBeVisible();
```

---

## 3. `attr:` prefix in Leptos view! macro renders literally in SSR

**Problem:** In Leptos 0.8, `attr:` is the syntax for arbitrary/hyphenated attributes
in the `view!` macro. For known HTML attributes (`disabled`, `class`, etc.), Leptos strips
the prefix correctly. But for custom attributes like `data-*`, the `attr:` prefix is
output **verbatim** in the SSR HTML:

```html
<!-- Wrong — attr: rendered literally -->
<button attr:data-color="slate" attr:data-active="true">

<!-- Correct -->
<button data-color="slate" data-active="true">
```

Because the browser stores `attr:data-color` as a literal attribute name, the Playwright
selector `[data-color="slate"]` never matches.

**Fix:** Write `data-*` attributes **without** the `attr:` prefix in the `view!` macro.
Despite containing a hyphen, Leptos's macro parser handles `data-*` names directly:

```rust
// Bad
attr:data-color=name
attr:data-active=move || (active.get() == name).to_string()

// Good
data-color=name
data-active=move || (active.get() == name).to_string()
```

---

## 4. Multiple demo instances on the same page

**Problem:** Documentation pages often render a component twice — once in the main demo
block and once further down the page. Playwright strict mode rejects locators that match
more than one element.

**Symptom:** `Error: strict mode violation: locator(...) resolved to 2 elements`

**Fix:** Always use `.first()` on locators in page objects, and combine attribute
selectors to narrow matches (e.g., `[data-active="true"][data-color]` excludes unrelated
elements that also use `data-active`):

```typescript
swatch(color: string): Locator {
  return this.page.locator(`[data-color="${color}"]`).first();
}

async expectActiveColor(color: string): Promise<void> {
  await expect(
    this.page.locator(`[data-color="${color}"][data-active="true"]`).first()
  ).toBeVisible();
}
```

---

## 5. Running the tests

```bash
# Start the dev server first
LEPTOS_SITE_ADDR="127.0.0.1:4000" LEPTOS_RELOAD_PORT="4001" cargo leptos watch --hot-reload

# Run all tests
cd e2e && BASE_URL="http://127.0.0.1:4000" pnpm test

# Run a single spec
BASE_URL="http://127.0.0.1:4000" pnpm test tests/hooks/use-history.spec.ts
```
