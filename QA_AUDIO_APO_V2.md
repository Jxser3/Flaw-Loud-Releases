# QA — APO Compatibility Engine 2.0

## Static checks completed in packaging environment

- `package.json` parsed successfully.
- `src-tauri/tauri.conf.json` parsed successfully.
- `src-tauri/keyauth.json` parsed successfully (KeyAuth remains separate from Connected Platform).
- `src-tauri/Cargo.toml` parsed successfully.
- TypeScript parser completed without new syntax errors; this packaging environment does not contain the project's npm dependencies, so module/type resolution cannot complete here.
- Rust/Cargo compilation cannot be executed in this packaging environment because Cargo/Rust are not installed. Use `RUN_FLAW_LOUD.bat` / the Windows release build for the native compile test.

## APO overrange design check

The supplied problematic recording contained decoded sample peaks around 1.65 full-scale. With the new default APO base reserve (10 dB), Clean-profile input gain and APO Input Guard 2.0, the guard calculation requires approximately 7.6 dB additional automatic trim, for about 17.6 dB total input trim. That projects the pre-dynamics peak to the 0.52 guard target instead of feeding the overrange signal into compression/saturation.

This is a design-level guard calculation, not a substitute for listening to the newly compiled native build on the user's actual Equalizer APO routing.

## Required listening test on Windows

1. Use the exact APO set that sounded covered/broken in the supplied video.
2. Start Flaw Loud with Engine OFF.
3. Enable `APO Compatibility Engine 2.0` and set Base Headroom to 10 dB.
4. Start Engine and speak at the same level as the bad recording.
5. Confirm APO Total Trim rises as needed.
6. Confirm APO Input reports SAFE/GUARDED instead of sustained HOT.
7. Watch limiter GR: sustained heavy reduction should be substantially lower than the previous build.
8. Compare A/B processed vs raw/APO signal.
9. Test Clean, Loud and Competition profiles before changing Base Headroom.
