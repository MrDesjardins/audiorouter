// Fresh packaged profiles show the goal picker after the canvas is ready.
// Decline it through the UI so smoke navigation cannot click behind a modal.
export async function dismissFirstLaunchGoal(page, timeout) {
  const dialog = page.getByRole("dialog", {
    name: "What do you want to do?",
    exact: true,
  });
  await dialog.waitFor({ state: "visible", timeout });
  await dialog.getByRole("button", { name: "Not now", exact: true }).click();
  await dialog.waitFor({ state: "hidden", timeout });
}
