# Repack2Proton Cloud KB (Knowledge Base)

This folder contains the documentation and sample files for the Cloud Knowledge Base, which powers the intelligent auto-configuration in R2P.

## 🌟 How it works (v1.6.8 Update)
1. **Local Priority:** The application first checks for `cloud/games.sample.json` to allow for offline usage or custom local overrides.
2. **Cloud Sync:** It then fetches the latest `presets.json` from the official GitHub repository (defined in `src/main.rs`).
3. **Database Integration:** The JSON data is imported into the local SQLite database (`factory.db`) under the `cloud_presets` table.
4. **Auto-Apply:** When a game is selected, R2P automatically applies:
    - Optimized Wine requirements (DXVK, VC++, XAudio).
    - Architecture (32/64-bit).
    - **Recommended Proton Version** (e.g., suggesting GE-Proton if specified in the KB).

## 🛠️ How to Contribute
To add a new game preset to the global database:
1. Fork the main repository.
2. Add your game entry to `presets.json` following the format below.
3. Submit a Pull Request.

## 📝 Format Reference
```json
[
  {
    "id": "game-slug",
    "name": "Full Game Name",
    "dxvk": true,
    "xaudio": false,
    "vcrun": true,
    "is_64bit": true,
    "proton": "GE-Proton",
    "preferred_exe": "bin/game.exe"
  }
]
```

## ⚙️ Hosting your own KB
1. Create a public repository or a Gist on GitHub.
2. Place a `presets.json` file in it.
3. Update the URL in `src/main.rs` (or future settings menu) to point to the "Raw" version of your file.

---
*Part of the Repack2Proton Ultimate Ecosystem.*
