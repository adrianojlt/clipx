import { invoke } from "@tauri-apps/api/core";

export const checkForUpdate = () => invoke("check_for_update");
