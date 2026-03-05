# Repack2Proton-RS - Progress Report

## Date: 2026-03-05 20:10:00 CET (Update checker + app shortcut UX)

### ✅ UPDATE CHECKER UX
- **Startup version probe:** aplikacja sprawdza `version.txt` z repo (`raw.githubusercontent.com`) przy starcie.
- **In-app banner:** jeśli dostępna jest nowsza wersja niż lokalna (`v1.0.2`), GUI pokazuje żółty baner z przyciskiem `POBIERZ` do strony Releases.
- **Quiet failure:** brak sieci lub błąd HTTP nie blokuje aplikacji.

### ✅ SETTINGS: APP SHORTCUT
- **Nowa opcja w Konfiguracji:** przycisk `DODAJ SKRÓT` instaluje/odświeża skrót Repack2Linux.
- **Desktop + menu parity:** wpis `.desktop` trafia do `~/.local/share/applications` oraz na pulpit użytkownika (z ikoną R2L).
- **WM class alignment:** `StartupWMClass=repack2linux` dla poprawnego mapowania ikony na dock/pasku.

## Date: 2026-03-05 18:20:00 CET (Welcome flow, sync messaging, icon cleanup)

### ✅ UX FIXES (WELCOME + SETTINGS)
- **Welcome startup policy:** ekran powitalny działa teraz bez warunku `first_launch_completed` i respektuje wyłącznie opcję `Pokazuj ekran powitalny przy starcie`.
- **No mixed messaging:** usunięto treści o przełączaniu Simple/Advanced z welcome overlay (PL/EN), aby opis odpowiadał aktualnemu, prostemu workflow.
- **Config persistence:** przełączniki welcome w Ustawieniach zapisują się stabilnie do configu.

### ✅ SYNC COMMUNICATION
- **Cloud sync messaging:** przy niedostępnym źródle cloud log pokazuje teraz ostrzeżenie z fallbackiem do lokalnej bazy zamiast twardego błędu.

### ✅ ICON IDENTITY
- **Text-free icon:** SVG ikony aplikacji i instalatora została uproszczona do samego symbolu/tła (bez napisu `R2L`), zgodnie z nowym stylem.
- **Shortcut refresh:** generator skrótu odświeża plik ikony przy starcie, żeby zmiana stylu była od razu widoczna po uruchomieniu nowej wersji.

## Date: 2026-03-05 16:45:00 CET (Portable reliability & zero-click launch flow)

### ✅ PORTABLE/SH LAUNCH FLOW (NO-TOUCH UX)
- **Domyślny entrypoint:** `play_auto.sh` jest używany jako główny start dla portable i desktop shortcutów (fallback do `play.sh` tylko awaryjnie).
- **Auto fallback:** `play_auto.sh` automatycznie ponawia start w trybie Safe (`--safe`) po błędzie uruchomienia, bez ręcznej ingerencji użytkownika.
- **Installer parity:** instalator `.sh` i GUI instalatora preferują `play_auto.sh`, więc zachowanie jest spójne między portable i SFX.

### ✅ BLACK SCREEN HARDENING
- **Auto-detekcja błędów DXVK:** log parser wykrywa sygnatury typu `VK_KHR_EXTERNAL_MEMORY_WIN32` / `Failed to create shared resource`.
- **Persisted Safe preference:** po wykryciu problemu aplikacja automatycznie przełącza profil gry na `no_dxvk`, a zapis learned profile utrwala to na kolejne buildy/eksporty.
- **Conservative launcher env:** uproszczono agresywne tuningi środowiska w `play.sh`, aby zachowanie portable było bliższe testowi uruchamianemu z aplikacji.

### ✅ PORTABILITY FIXES
- **Working directory parity:** launcher portable uruchamia EXE przez ścieżkę absolutną względem `SCRIPT_DIR`, eliminując różnice względem trybu testowego.
- **Save-link resilience:** naprawa relatywnych symlinków `AppData/Local` oraz runtime-repair dla martwych linków po przeniesieniu paczki między komputerami/dyskami.
- **Prefix guardrails:** `play.sh` jasno zgłasza brak/korupcję hive (`system.reg/user.reg/userdef.reg`) zamiast kończyć się czarnym ekranem bez informacji.

### ✅ OPS / GIT
- Zmiany zgrupowane w commit `2a80c19` i wypchnięte na branch `clean-main`.
- Branch gotowy pod PR: `https://github.com/KrystianG06/Repack2Proton/pull/new/clean-main`

## Date: 2026-03-05 14:25:00 CET (Community Sync & UI Language Cleanup)

### ✅ LEARNING & COMMUNITY SYNC
- **Auto-learn po sukcesie:** profil gry (biblioteki, architektura, proton, hint EXE/prefix) zapisuje się po udanej produkcji, niezależnie od kliknięć w końcowym oknie eksportu.
- **JSON profile memory:** dodano `~/.config/repack2proton/learned-profiles.json` z ładowaniem po `source_path` oraz `game_id`, dzięki czemu ta sama gra odzyskuje ustawienia nawet z innego folderu.
- **Community upsert:** automatyczny update `presets.json` i `cloud/games.sample.json` po zakończonej produkcji.
- **GitHub API sync:** opcjonalny push przez `R2P_GITHUB_TOKEN` (`R2P_GITHUB_REPO` / `R2P_GITHUB_BRANCH`), bez ręcznego edytowania JSON.

### ✅ RELIABILITY (QUEUE / RETRY POLICY)
- **Repo root resolver:** sync nie używa już ślepo `.`; wykrywa root po strukturze repo (`presets.json` + `cloud/games.sample.json`) i wspiera `R2P_COMMUNITY_DB_ROOT`.
- **Retry queue:** nieudane remote sync trafia do `~/.config/repack2proton/community-sync-queue.json`.
- **Backoff + TTL + limit:** kolejka ma exponential backoff, limit prób i TTL (stare/zbyt wiele prób są czyszczone), żeby uniknąć wiecznych błędów.
- **Startup recovery:** aplikacja przy starcie automatycznie próbuje opróżnić kolejkę.

### ✅ UI / UX POLISH
- **Community status card:** nowy panel w Settings pokazuje: status kolejki, liczbę prób, last retry, last error, repo root i stan tokena, plus przycisk `RETRY QUEUE NOW`.
- **Polish/English cleanup:** naprawiono mieszanie języków w zakładce Settings — etykiety przechodzą przez `tr()` i są spójne dla PL/EN.
- **Skróty pulpitu (SFX installer):** instalator używa `xdg-user-dir DESKTOP` (fallback `~/Desktop`), ustawia `.desktop` jako executable i poprawia naming, więc skróty tworzą się stabilniej na różnych desktopach.

### ✅ ICON IDENTITY
- `r2p-icon.svg` jest zapisywana po instalacji i używana w desktop entries jako spójna ikona motywu R2P.

## Date: 2026-03-04 19:23:11 CET (Stability & Intelligence Update)

### ✅ SFX & INSTALLER ULTIMATE
- **Robust Extraction:** Fixed busy-loops and pipe breaks in `installer_gui`.
- **System Doctor:** Added 32-bit dependency detection (Vulkan/GL) with one-click "Fix System" button in SFX.
- **FS Stability:** Added `sync` and explicit `chmod` logic for reliable installation on HDD/NTFS partitions.
- **Space-Safe:** Improved quoting for paths containing spaces.
- **Marker Uniqueness:** Switched to `R2P_GUI_BIN_START` to avoid collisions with game data.
- **Registy Integrity Guard:** Export now validates `system.reg`, `user.reg`, `userdef.reg` before running `tar` (no `--ignore-failed-read`), and the installer GUI double-checks the unpacked prefix so corrupted registry hives abort with a clear error instead of launching Wine with missing kernel32.

### ✅ DATABASE & AUTOMATION
- **Intelligence:** Implemented Fuzzy Matching for game names (normalization of GTA, IV, etc.).
- **Full-Auto:** Database now controls all engine parameters (PhysX, XACT, D3DX9, VCRun versions).
- **Proton Tracking:** Added `ProtonSource` logging to monitor selection logic.
- **Cloud Sync:** Updated to point directly to `cloud/games.sample.json` in the main repository.
- **UI Feedback:** Added sidebar indicator for loaded presets count.
- **Offline first:** Sync routines and dependency checks rely on local data, so R2P never requests Steam tokens or cloud access during prefix detection.

### ✅ UI & UX
- **Palette:** Cały interfejs korzysta z niebiesko-czerwono-szarej palety, stałych promieni i czcionki Noto Sans, a przyciski są spójne bez emoji.
- **Dialogs:** Modalne okna blokują główne komponenty, logi używają tej samej palety, a eksport pokazuje informacje o czcionce i stylach również w README i wygenerowanym `play.sh`.
- **Responsywność:** kafelki Fabryki, narzędzi i ustawień skalują się, a pasek postępu zawsze pozostaje widoczny i animowany podczas asynchronicznych zadań.
- **Audit i szybki dry run:** eksport dopisuje SHA256 komponentów, modal pokazuje je użytkownikowi, a dry run wykonuje szybką weryfikację tylko prefixu + `play.sh`/README (bez pakowania całej paczki), dzięki czemu diagnostyka jest kilkukrotnie szybsza.
- **Test coverage:** dodano jednostkowe testy `Installer::search_existing_base_prefix`, `prefix_score`, generatora `play.sh` oraz `ShortcutManager::create_desktop_shortcut` – `cargo test` weryfikuje tworzenie skrótów i skryptów.
- **Ikona instalatora:** po instalacji zapisujemy `r2p-icon.svg` obok `play.sh`, a wpisy `.desktop` używają tej ścieżki, więc skróty dostają identyczną ikonę z motywu.

### ✅ ENGINE & DETECTOR
- **Expanded Requirements:** Added support for legacy dependencies in the detector and production engine.
- **Stability Fixes:** Resolved ownership and lifetime issues in async tasks and streams.
- **Asset Recovery:** Fixed icon extraction and binary scoring logic.
- **Prefiks ready:** Nowy mechanizm skanuje katalogi Wine/Lutris/Steam, ocenia `WINEARCH` i daty modyfikacji, a następnie kopiuje gotowe prefixy zamiast budować od zera.
- **Prefiks ready:** Nowy mechanizm skanuje katalogi Wine/Lutris/Steam (w tym `~/.var/app/*/.wine` i `compatdata/*/pfx`), ocenia `WINEARCH` i daty modyfikacji, a następnie kopiuje gotowe prefixy zamiast budować od zera; decyzje zapisujemy w `~/.config/repack2proton/prefix-selection.json`, a ostatni używany prefix dla gry trafia do `prefix-records.json`, więc kolejne produkcje od razu odnajdą sprawdzony zestaw.

## Date: 2026-03-04 20:45:00 CET (UI polish & prefix heuristics)

### ✅ PREFIX HEURISTICS
- **Atlas środowisk:** wzbogacono `prefix_roots` o zmienne `WINEPREFIX`, `LUTRIS_PREFIX`, `LUTRIS_PREFIXES` i `R2P_PREFIX_ROOT`, aby centralnie uwzględniać standardowe katalogi oraz wskazane ręcznie ścieżki.
- **Ranking bezpieczeństwa:** `prefix_score` dodaje punkty za obecność `drive_c/windows/system32/kernel32.dll`, zgodność `WINEARCH`, oraz wzmianki o Protonie w `system.reg` i `user.reg`, co przekłada się na trafniejszy wybór prefabrykowanych środowisk.

### ✅ UI & INSTALLER POLISH
- **Spójność stylów:** logi, panele i przyciski trzymają się jednej palety niebiesko-czerwono-szarej, gradientowe tła są bardziej stonowane, a modale blokują kliknięcia w tle.
- **Instalator GUI:** `accent_button_style` i `ghost_button_style` gwarantują jednolite promienie, cień i kolory, a `play.sh` przypomina o czcionce i motywie przy uruchamianiu.

### ✅ DOKUMENTACJA I LOGI
- **Czas w dokumentach:** README i PROGRESS zawierają dokładny znacznik daty/godziny oraz zapis „całkowitego postępu”, a wygenerowany `play.sh` dokumentuje używaną czcionkę/temat.

---

## TO DO
- [ ] Implement Delta-Sync for game files (rsync-like logic).
- [ ] Add Steam Deck optimized presets.
- [ ] Visual polish for the advanced engine parameters tab.

## Date: 2026-03-04 21:30:00 CET (Skip-cleanup & prefix polish)

### ✅ AUDYTY I PRYWATNOŚĆ
- **Skip cleanup** w panelu eksportu pozwala zachować foldery robocze i prefixy, a logi pokazują, gdzie je zostawiono.
- Bez instalatora lub przy dry runie wypełniamy `ExportAudit`, więc modal zawsze pokazuje hash prefixu/`play.sh/README`.

### ✅ UI I MODALE
- Polskie nagłówki eksportu, nowe gradientowe przyciski i ekran postępu pozostają w palecie czerwono-niebiesko-szarej bez żadnych emotek.
- Dialogi eksportu blokują tło, przyciski mają subtelny cień, a `view_export_success` prezentuje czytelny status z guzikami „Otwórz folder” i „Zamknij”.

### ✅ PREFIKSY FALBACK
- Skrypt `Installer::collect_export_audits` dzieli logikę minutową, nowa metoda `best_recorded_prefix` szuka najlepszych prefixów z `prefix-records.json`, klonuje je, oczyszcza i zapisuje do `prefix-selection.json`, więc nigdy nie wracamy do brudnego prefixu.
