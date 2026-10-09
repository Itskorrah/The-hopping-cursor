import { test, expect } from "@playwright/test";

test("protected startup keeps native chrome available without sample data", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.addInitScript(() => {
    Object.defineProperty(window, "__TAURI_EVENT_PLUGIN_INTERNALS__", { value: { unregisterListener: () => {} } });
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: {
      metadata: { currentWebview: { label: "main" }, currentWindow: { label: "main" } },
      transformCallback: () => 1,
      unregisterCallback: () => {},
      invoke: async (command: string) => {
        if (command === "snapshot") throw "Waiting for protected pairing identity approval";
        if (command === "quit_app") {
          document.documentElement.dataset.quitRequested = "true";
          return;
        }
        return 1;
      },
    } });
  });
  await page.goto("/");
  await expect(page.getByText("Waiting for protected pairing identity approval")).toBeVisible();
  await expect(page.getByText("Studio Mac")).toHaveCount(0);
  await expect(page.getByText("Design PC")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Pause", exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "Check again" }).click();
  await page.getByRole("button", { name: "Quit application" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-quit-requested", "true");
  expect(errors).toEqual([]);
});
