import { invoke } from "@tauri-apps/api/core";

export const getHistory = () => invoke("get_history");

export const getPinned = () => invoke("get_pinned");

export const getGlobalPinned = () => invoke("get_global_pinned");

export const getClipboard = () => invoke("get_clipboard");

export const getSetting = (key) => invoke("get_setting", { key });

export const setSetting = (key, value) => invoke("set_setting", { key, value });

export const pinItem = (content) => invoke("pin_item", { content });

export const deleteHistoryItem = (id) => invoke("delete_history_item", { id });

export const unpinItem = (id) => invoke("unpin_item", { id });

export const updatePinnedDescription = (id, description) => invoke("update_pinned_description", { id, description });

export const togglePinnedHidden = (id) => invoke("toggle_pinned_hidden", { id });

export const reorderPinned = (items) => invoke("reorder_pinned", { items });

export const updateShortcut = (shortcut) => invoke("update_shortcut", { shortcut });

export const applyWindowSize = () => invoke("apply_window_size");

export const logError = (level, message) => invoke("log_frontend_error", { level, message });

export const getSessions = () => invoke("get_sessions");

export const createSession = (name) => invoke("create_session", { name });

export const deleteSession = (id) => invoke("delete_session", { id });

export const activateSession = (id) => invoke("activate_session", { id });

export const reorderSessions = (items) => invoke("reorder_sessions", { items });

export const pinItemToSession = (content, sessionId, description) => invoke("pin_item_to_session", { content, sessionId, description });

export const listOpenApps = () => invoke("list_open_apps");

export const focusApp = (id) => invoke("focus_app", { id });
