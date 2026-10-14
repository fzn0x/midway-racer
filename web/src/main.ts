import "./styles.css";

import init, { supportedGames } from "./wasm/midway_racer_web.js";

const status = document.querySelector<HTMLElement>("#loader-status");

async function main(): Promise<void> {
  await init();
  const titles = supportedGames().map((game) => game.title);
  if (status) {
    status.textContent = `Ready to run ${titles.join(", ")}.`;
  }
}

main().catch((error: unknown) => {
  console.error(error);
  if (status) {
    status.textContent = `The emulator could not start: ${String(error)}`;
  }
});
