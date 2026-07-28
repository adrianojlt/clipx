import { invoke } from "@tauri-apps/api/core";

export const exportData = (path) => invoke("export_data", { path });

export const previewImport = (path) => invoke("preview_import", { path });

export const importData = (path) => invoke("import_data", { path });
