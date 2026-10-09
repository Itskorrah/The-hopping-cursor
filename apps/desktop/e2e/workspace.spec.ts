import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
let errors: string[] = [];
test.beforeEach(async ({ page }) => {
  errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("response", (r) => {
    if (r.status() >= 500) errors.push(`${r.status()} ${r.url()}`);
  });
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Your desk, connected." }),
  ).toBeVisible();
});
test.afterEach(() => expect(errors).toEqual([]));
test("workspace clearly labels preview and sharing can be paused", async ({
  page,
}) => {
  await expect(
    page.getByText(
      "Interface preview with sample computers. Real sharing runs in the desktop app.",
    ),
  ).toBeVisible();
  await page.getByRole("button", { name: "Pause", exact: true }).click();
  await expect(page.getByText("Sharing paused")).toBeVisible();
  await page.reload();
  await expect(
    page.getByRole("button", { name: "Resume", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Resume", exact: true }).click();
  await expect(page.getByText("Workspace ready")).toBeVisible();
});
test("sharing preferences persist across reload", async ({ page }) => {
  await page.getByRole("button", { name: "Preferences", exact: true }).click();
  const toggle = page.getByRole("switch", { name: "Text clipboard" });
  await expect(toggle).toBeChecked();
  await toggle.click();
  await expect(toggle).not.toBeChecked();
  await page.reload();
  await page.getByRole("button", { name: "Preferences", exact: true }).click();
  await expect(toggle).not.toBeChecked();
});

test("two PC monitors have distinct primary labels and OS order can repair a reversed layout", async ({
  page,
}) => {
  await page.getByRole("button", { name: "Pause", exact: true }).click();
  await page.getByRole("button", { name: "Resume", exact: true }).click();
  await page.evaluate(() => {
    const key = "hopping-cursor-interface-preview-v1";
    const state = JSON.parse(localStorage.getItem(key)!);
    const pc = state.peers.find(
      (p: { device: { id: string } }) => p.device.id === "pc-preview",
    ).device;
    const primary = pc.displays[0];
    const right = {
      ...primary,
      id: "right",
      primary: false,
      bounds: { ...primary.bounds, x: 1920 },
    };
    pc.displays = [right, primary];
    state.layout.find(
      (d: { device_id: string }) => d.device_id === pc.id,
    ).rect.x = 3840;
    state.layout.push({
      device_id: pc.id,
      display_id: right.id,
      rect: { x: 1920, y: 0, width: 1920, height: 1080 },
      hidden: false,
    });
    localStorage.setItem(key, JSON.stringify(state));
  });
  await page.reload();
  const primary = page.getByRole("button", {
    name: "Arrange Design PC display Display 1 · Primary",
    exact: true,
  });
  const right = page.getByRole("button", {
    name: "Arrange Design PC display Display 2",
    exact: true,
  });
  await expect(primary).toBeVisible();
  await expect(right).toBeVisible();
  await page
    .getByRole("button", { name: "Use OS display order", exact: true })
    .click();
  await page.getByRole("button", { name: "Save layout", exact: true }).click();
  await expect(page.getByText("Display arrangement saved")).toBeVisible();
  await page.reload();
  const primaryBox = await primary.boundingBox(),
    rightBox = await right.boundingBox();
  expect(primaryBox!.x).toBeLessThan(rightBox!.x);
});
test("pairing requires an explicit fingerprint comparison", async ({
  page,
}) => {
  await page.getByRole("button", { name: "Computers", exact: true }).click();
  await page.getByRole("button", { name: "Pair computer" }).click();
  const approve = page.getByRole("button", { name: "Trust this computer" });
  await expect(approve).toBeDisabled();
  await page
    .getByRole("checkbox", {
      name: "I compared the codes on both computers and they match.",
    })
    .check();
  await expect(approve).toBeEnabled();
  await approve.click();
  await expect(page.getByRole("dialog")).not.toBeVisible();
  await expect(
    page.getByText("Computer approved. Approve on the other computer too."),
  ).toBeVisible();
});
test("keyboard arrangement rejects overlap and retains unsaved edits", async ({
  page,
}) => {
  await page.getByRole("button", { name: "Arrange Design PC display" }).focus();
  await page.keyboard.press("Shift+ArrowLeft");
  await page.getByRole("button", { name: "Save layout" }).click();
  await expect(
    page.getByText("Visible displays cannot overlap; drag them edge to edge"),
  ).toBeVisible();
  await expect(page.getByText("Unsaved changes")).toBeVisible();
  await page.getByRole("button", { name: "Reset", exact: true }).click();
  await expect(page.getByText("Unsaved changes")).not.toBeVisible();
});
test("hidden display can be saved and restored", async ({ page }) => {
  await page.getByRole("button", { name: "Hide Design PC display" }).click();
  await page.getByRole("button", { name: "Save layout" }).click();
  await expect(
    page.getByRole("button", { name: "Show Design PC" }),
  ).toBeVisible();
  await page.reload();
  await expect(
    page.getByRole("button", { name: "Show Design PC" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Show Design PC" }).click();
  await page.getByRole("button", { name: "Save layout" }).click();
  await expect(
    page.getByRole("button", { name: "Arrange Design PC display" }),
  ).toBeVisible();
});
test("all pages pass accessibility checks", async ({ page }) => {
  for (const name of [
    "Workspace",
    "Computers",
    "File transfers",
    "Preferences",
    "Diagnostics",
  ]) {
    await page.getByRole("button", { name, exact: true }).click();
    const result = await new AxeBuilder({ page }).analyze();
    expect(
      result.violations.map((v) => ({
        id: v.id,
        nodes: v.nodes.map((n) => n.target),
      })),
      name,
    ).toEqual([]);
  }
});
test("small screens stay within the viewport", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(
    page.getByRole("heading", { name: "Your desk, connected." }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.getByRole("button", { name: "Preferences", exact: true }).click();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
});

test("layout profiles restore saved display visibility", async ({ page }) => {
  await page.getByRole("button", { name: "Save as profile" }).click();
  await page.getByLabel("Profile name").fill("Home desk");
  await page.getByRole("button", { name: "Save profile", exact: true }).click();
  await page.getByRole("button", { name: "Hide Design PC display" }).click();
  await page.getByRole("button", { name: "Save layout" }).click();
  await page.getByLabel("Layout profile").selectOption("");
  await page.getByLabel("Layout profile").selectOption("Home desk");
  await expect(
    page.getByRole("button", { name: "Arrange Design PC display" }),
  ).toBeVisible();
});
test("computer sharing permissions are controllable", async ({ page }) => {
  await page.getByRole("button", { name: "Computers", exact: true }).click();
  await page
    .getByRole("button", { name: "Sharing permissions for Design PC" })
    .click();
  const toggle = page.getByRole("switch", { name: "Allow files" });
  await expect(toggle).toBeChecked();
  await toggle.click();
  await expect(toggle).not.toBeChecked();
  await page.getByRole("button", { name: "Done", exact: true }).click();
});
test("display shortcuts can be assigned", async ({ page }) => {
  await page.getByRole("button", { name: "Arrange Design PC display" }).focus();
  await page.getByLabel("Hop shortcut").selectOption("30");
  await expect(page.getByLabel("Hop shortcut")).toHaveValue("30");
  await page.getByRole("button", { name: "Jump here" }).click();
  await expect(page.getByText("Input moved to the display")).toBeVisible();
});

test("file clipboard is opt-in and persists", async ({ page }) => {
  await page.getByRole("button", { name: "Preferences", exact: true }).click();
  const toggle = page.getByRole("switch", {
    name: "File clipboard",
    exact: true,
  });
  await expect(toggle).not.toBeChecked();
  await toggle.click();
  await page.reload();
  await page.getByRole("button", { name: "Preferences", exact: true }).click();
  await expect(toggle).toBeChecked();
  await expect(
    page.getByText(/Newer clipboard contents are preserved/),
  ).toBeVisible();
});

test("shared layout review preserves unsaved edits and requires applying", async ({
  page,
}) => {
  await page.getByRole("button", { name: "Pause", exact: true }).click();
  await page.getByRole("button", { name: "Resume", exact: true }).click();
  await page.evaluate(() => {
    const key = "hopping-cursor-interface-preview-v1";
    const state = JSON.parse(localStorage.getItem(key)!);
    state.layout_offers = [
      {
        peer_id: "pc-preview",
        layout: state.layout.map((l: { rect: { y: number } }) => ({
          ...l,
          rect: { ...l.rect, y: 300 },
        })),
      },
    ];
    localStorage.setItem(key, JSON.stringify(state));
  });
  await page.reload();
  await expect(page.getByText("Arrangement from Design PC")).toBeVisible();
  const screen = page.getByRole("button", { name: /Arrange Studio Mac/ });
  await screen.focus();
  await page.keyboard.press("ArrowRight");
  await expect(
    page.getByRole("button", { name: "Apply arrangement" }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Reset", exact: true }).click();
  await page.getByRole("button", { name: "Apply arrangement" }).click();
  await expect(page.getByText("Shared arrangement applied")).toBeVisible();
  const saved = await page.evaluate(() =>
    JSON.parse(localStorage.getItem("hopping-cursor-interface-preview-v1")!),
  );
  expect(
    saved.layout.every((l: { rect: { y: number } }) => l.rect.y === 300),
  ).toBe(true);
  expect(saved.layout_offers).toEqual([]);
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
});
