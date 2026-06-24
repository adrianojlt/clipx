import { getVersion } from '@tauri-apps/api/app';

const version = await getVersion();
document.getElementById('version').textContent = `v${version}`;
