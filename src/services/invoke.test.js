import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";

const tauriInvoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args) => tauriInvoke(...args),
}));

const { invoke } = await import("./invoke");

// The wrapper sleeps between attempts, so the retry tests would otherwise spend
// real time waiting. Fake timers plus `runAllTimersAsync` let the whole retry
// loop resolve immediately.
beforeEach(() => {
  tauriInvoke.mockReset();
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

// Drives a call that needs timers to advance before it settles.
const settle = async (promise) => {
  const done = promise.then(
    (value) => ({ value }),
    (error) => ({ error }),
  );
  await vi.runAllTimersAsync();
  return done;
};

const notManaged = () =>
  new Error(
    "state not managed for field `state` on command `list_open_apps`. " +
      "You must call `.manage()` before using this command",
  );

describe("invoke", () => {

  it("passes the command and args straight through when nothing fails", async () => {
    tauriInvoke.mockResolvedValue({ apps: [] });

    const { value } = await settle(invoke("list_open_apps", { id: "7" }));

    expect(value).toEqual({ apps: [] });
    expect(tauriInvoke).toHaveBeenCalledTimes(1);
    expect(tauriInvoke).toHaveBeenCalledWith("list_open_apps", { id: "7" });
  });

  // The startup race: the webview beats `app.manage(AppState)` and the command
  // boundary rejects the call before any Rust runs.
  it("retries a not-managed rejection until the backend catches up", async () => {
    tauriInvoke
      .mockRejectedValueOnce(notManaged())
      .mockRejectedValueOnce(notManaged())
      .mockResolvedValue("ready");

    const { value } = await settle(invoke("get_history"));

    expect(value).toBe("ready");
    expect(tauriInvoke).toHaveBeenCalledTimes(3);
  });

  it("gives up after a bounded number of attempts", async () => {
    tauriInvoke.mockRejectedValue(notManaged());

    const { error } = await settle(invoke("get_history"));

    expect(String(error)).toContain("state not managed");
    // The first attempt plus MAX_RETRIES.
    expect(tauriInvoke).toHaveBeenCalledTimes(11);
  });

  // Everything else must surface on the first attempt: retrying a real backend
  // failure would only delay the error the caller already handles.
  it("does not retry any other error", async () => {
    tauriInvoke.mockRejectedValue(new Error("Database error: disk I/O error"));

    const { error } = await settle(invoke("get_history"));

    expect(String(error)).toContain("disk I/O error");
    expect(tauriInvoke).toHaveBeenCalledTimes(1);
  });

  // Tauri rejects with a plain string, not an Error, for `Result<T, AppError>`.
  it("recognizes the race when it arrives as a bare string", async () => {
    tauriInvoke
      .mockRejectedValueOnce("state not managed for field `state`")
      .mockResolvedValue("ready");

    const { value } = await settle(invoke("get_pinned"));

    expect(value).toBe("ready");
    expect(tauriInvoke).toHaveBeenCalledTimes(2);
  });
});
