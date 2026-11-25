const status = document.querySelector(".startup");
try {
  const version = new URL(import.meta.url).search;
  const { default: init } = await import(`./pkg/planner.js${version}`);
  await init({ module_or_path: new URL(`./pkg/planner_bg.wasm${version}`, import.meta.url) });
  status?.remove();
} catch (error) {
  if (status) status.textContent = "The page could not load. Refresh to try again.";
  console.error(error);
}
