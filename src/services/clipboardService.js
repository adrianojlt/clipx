import { invoke } from "@tauri-apps/api/core";

export const getHistory = () => invoke("get_history");

export const getPinned = () => invoke("get_pinned");

export const getClipboard = () => invoke("get_clipboard");

export const getSetting = (key) => invoke("get_setting", { key });

export const setSetting = (key, value) => invoke("set_setting", { key, value });

export const pinItem = (content) => invoke("pin_item", { content });

export const deleteHistoryItem = (id) => invoke("delete_history_item", { id });

export const unpinItem = (id) => invoke("unpin_item", { id });

export const updatePinnedDescription = (id, description) =>
  invoke("update_pinned_description", { id, description });

export const togglePinnedHidden = (id) => invoke("toggle_pinned_hidden", { id });

export const reorderPinned = (items) => invoke("reorder_pinned", { items });

export const updateShortcut = (shortcut) => invoke("update_shortcut", { shortcut });

export const applyWindowSize = () => invoke("apply_window_size");
