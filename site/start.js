const status = document.querySelector(".startup");
try {
  const { default: init } = await import("./pkg/planner.js");
  await init();
  status?.remove();
} catch (error) {
  if (status) status.textContent = "The page could not load. Refresh to try again.";
  console.error(error);
}
