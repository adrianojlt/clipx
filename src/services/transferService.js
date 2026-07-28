import { invoke } from "@tauri-apps/api/core";

// Tauri serializes AppError as "<Kind> error: <detail>". The kind is internal
// taxonomy, so only the detail is worth putting in front of a user.
const stripErrorKind = (e) => String(e).replace(/^[A-Za-z]+ error: /, "");

const call = async (command, path) => {
  try {
    return await invoke(command, { path });
  } catch (e) {
    throw new Error(stripErrorKind(e));
  }
};

export const exportData = (path) => call("export_data", path);

// The backend speaks snake_case. Mapping here keeps the serde field names from
// reaching components, so renaming a Rust field breaks one file instead of two.
export const previewImport = async (path) => {

  const preview = await call("preview_import", path);

  return {
    version: preview.version,
    exportedAt: preview.exported_at,
    fileHistory: preview.file_history,
    historyToImport: preview.history_to_import,
    fileSessions: preview.file_sessions,
    filePinned: preview.file_pinned,
    currentHistory: preview.current_history,
    currentSessions: preview.current_sessions,
    currentPinned: preview.current_pinned,
  };
};

export const importData = (path) => call("import_data", path);
