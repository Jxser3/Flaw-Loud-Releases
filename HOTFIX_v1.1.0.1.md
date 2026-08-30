# Flaw Loud v1.1.0.1 RC — Updater Config Hotfix

Internal SemVer: `1.1.0-rc.2`.

Fixed startup crash:

`PluginInitialization("updater", "Error deserializing plugins.updater ... invalid type: null")`

The updater plugin now receives a valid Tauri configuration object during startup. The actual signed updater endpoint and public key remain runtime-configurable through `src-tauri/updater.json` / `CONFIGURE_PRODUCTION.bat`.

No audio/DSP or Connected Platform behavior changed.
