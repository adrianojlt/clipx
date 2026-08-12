import { invoke as tauriInvoke } from "@tauri-apps/api/core";

// Tauri creates the windows declared in `tauri.conf.json` before the Rust
// `setup` hook finishes, so the webview starts running its mount effects while
// `init_app_state` is still opening the database. A command that takes
// `State<AppState>` invoked in that gap is rejected by Tauri's command boundary
// with "state not managed", before any of our Rust runs.
//
// The gap is microseconds wide and every state-taking command shares it, so
// which call loses is down to scheduling: it cost the apps list once, and the
// panel then stayed empty until the next refresh. Retrying that one error
// briefly closes it without moving the three window definitions out of the
// declarative config.
//
// Matched on the message rather than an `AppError` variant deliberately: the
// rejection happens above our error type, so there is nothing else to key on.
const NOT_MANAGED = "state not managed";

// 10 x 25 ms bounds the wait at 250 ms, far above the observed gap. A command
// still failing after that is failing for some other reason, and the error
// belongs on the caller rather than in a longer retry.
const MAX_RETRIES = 10;
const RETRY_DELAY_MS = 25;

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

// Drop-in `invoke` that rides out the startup race described above. Every
// service module goes through this rather than `@tauri-apps/api/core`.
export const invoke = async (command, args) => {

  for (let attempt = 0; ; attempt++) {

    try {
      return await tauriInvoke(command, args);
    } catch (e) {
      if (attempt >= MAX_RETRIES || !String(e).includes(NOT_MANAGED)) {
        throw e;
      }
      await sleep(RETRY_DELAY_MS);
    }
  }
};
