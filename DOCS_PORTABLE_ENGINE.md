# Repack2Linux: Dokumentacja Przenośnego Silnika (Portable Engine)

## 1. Wprowadzenie
Repack2Linux (R2L) to zaawansowany pipeline inżynieryjny służący do tworzenia samowystarczalnych (portable) paczek gier Windows dla systemów Linux. Niniejsza dokumentacja opisuje architekturę silnika "Zero Dependencies", który gwarantuje uruchomienie gry na dowolnej dystrybucji Linuxa bez konieczności instalowania zewnętrznych zależności (takich jak Wine, Winetricks czy biblioteki 32-bitowe).

---

## 2. Architektura "Zero Dependencies"

### 2.1. Koncepcja Bundlowania
W przeciwieństwie do tradycyjnych skryptów Lutris czy Steam, paczki R2L zawierają w sobie **kompletne środowisko wykonawcze (Runtime Environment)**.

Struktura wyeksportowanej paczki (Folder lub zawartość `.sh`):
```text
[Game_Name]_Portable/
├── game_files/          # Pliki gry (.exe, .dll, data)
├── pfx/                 # Skonfigurowany prefix Wine (rejestr, C:)
├── wine/                # [NOWOŚĆ] Pełny silnik Wine/Proton (binarki + liby)
├── play.sh              # Główny skrypt startowy (Intelligent Launcher)
├── play_auto.sh         # Wrapper z automatycznym trybem Safe Mode
├── adddesktopicon.sh    # Skrypt pomocniczy do tworzenia skrótów
├── icon.png             # Wyodrębniona ikona gry (PNG HQ)
└── r2l_brand.svg        # Ikona marki R2L (fallback)
```

### 2.2. Izolacja Bibliotek (LD_LIBRARY_PATH)
Kluczem do przenośności jest mechanizm izolacji bibliotek zaimplementowany w `play.sh`. Skrypt startowy nie polega na systemowym linkerze dynamicznym. Zamiast tego:
1.  Wykrywa obecność folderu `./wine`.
2.  Ustawia zmienną `LD_LIBRARY_PATH` na `./wine/lib` oraz `./wine/lib64`.
3.  Ustawia `WINELOADER` na `./wine/bin/wine`.

Dzięki temu gra korzysta **wyłącznie** z bibliotek dostarczonych w paczce (glibc, libvulkan, libX11 itd.), ignorując potencjalne braki lub konflikty w systemie hosta.

---

## 3. Inteligentny Launcher (`play.sh`)

Skrypt `play.sh` to mózg operacji. Nie jest to zwykły one-liner. Posiada wbudowaną logikę heurystyczną:

### 3.1. Wykrywanie Środowiska (Runtime Probe)
Przy każdym uruchomieniu skrypt wykonuje:
*   **GPU Check:** Sprawdza dostępność akceleracji OpenGL (`glxinfo`).
*   **Vulkan Check:** Sprawdza wsparcie dla Vulkan (`vulkaninfo` lub `ls /usr/lib/libvulkan.so`).
*   **Runtime Check:** Decyduje, czy użyć wbudowanego Wine (priorytet), czy systemowego (fallback).

### 3.2. Automatyczny Safe Mode
Jeśli wykrycie Vulkana zawiedzie (np. na starych kartach Intel HD lub przy braku sterowników), skrypt automatycznie:
1.  Wyłącza DXVK (`R2L_RENDERER="wined3d"`).
2.  Przełącza renderowanie na OpenGL (WineD3D).
3.  Informuje użytkownika o trybie awaryjnym w konsoli.

### 3.3. Izolacja Zapisów (Save Isolation)
Aby zachować charakter "Portable", zapisy gry (zazwyczaj w `%AppData%`) są przekierowywane do folderu `./r2p_userdata` wewnątrz paczki.
*   **Zasada działania:** Skrypt tworzy symlinki z `pfx/drive_c/users/steamuser/AppData` do `./r2p_userdata`.
*   **Korzyść:** Możesz przenieść folder z grą na inny komputer i zachowasz swoje save'y.

---

## 4. Instrukcja dla Twórcy (Repackera)

### 4.1. Jak stworzyć paczkę "Full Portable"?
1.  Otwórz R2L i wybierz folder źródłowy gry.
2.  W sekcji "Środowisko Bazowe" wybierz konkretną wersję GE-Proton (np. `GE-Proton8-25`).
    *   *Uwaga: Jeśli wybierzesz "System Wine (Default)", silnik NIE zostanie dołączony!*
3.  Skonfiguruj parametry (DXVK, VC++, itd.).
4.  Kliknij "Export Options" -> zaznacz "Instalator Unified SFX (.sh)".
5.  Uruchom eksport.

### 4.2. Weryfikacja
Po zakończeniu eksportu, wejdź do folderu wynikowego i sprawdź, czy folder `wine/` istnieje i zajmuje ok. 200-400 MB. Jeśli tak – paczka jest gotowa i niezależna.

---

## 5. Instrukcja dla Użytkownika Końcowego

### 5.1. Uruchamianie (Metoda "Kliknij i Graj")
1.  Pobierz plik `.sh` (np. `Cyberpunk_Portable.sh`).
2.  Nadaj uprawnienia wykonywania:
    *   Prawy przycisk myszy -> Właściwości -> Uprawnienia -> "Zezwól na wykonywanie pliku".
    *   Lub w terminalu: `chmod +x Cyberpunk_Portable.sh`.
3.  Uruchom plik.
4.  Instalator rozpakuje grę i automatycznie ją uruchomi.

### 5.2. Rozwiązywanie Problemów
Jeśli gra nie startuje, uruchom ją z terminala, aby zobaczyć diagnostykę R2L:
```bash
./Cyberpunk_Portable.sh
```
Szukaj komunikatów oznaczonych tagiem `[R2L]`.

**Tryb Bezpieczny (Safe Mode):**
Możesz wymusić tryb OpenGL (wolniejszy, ale bardziej kompatybilny) flagą:
```bash
./play.sh --safe
```

---

## 6. Specyfikacja Techniczna (Dla Developerów)

### Zmienne Środowiskowe R2L
| Zmienna | Opis | Wartość domyślna |
| :--- | :--- | :--- |
| `R2L_RENDERER` | Wybrany backend graficzny | `dxvk` lub `wined3d` |
| `PROTONPATH` | Ścieżka do zbundlowanego silnika | `./wine` |
| `WINEPREFIX` | Ścieżka do prefixu | `./pfx` |
| `WINEDLLOVERRIDES` | Nadpisania bibliotek DLL | Zależne od presetu |

### Struktura SFX
Instalator `.sh` składa się z:
1.  **Nagłówka Bash:** Logika dekompresji i GUI startowego.
2.  **Payloadu TAR.GZ:** Skompresowane pliki gry + prefix + wine.
3.  **Sumy Kontrolnej:** (Opcjonalnie) SHA256 na końcu pliku.

Mechanizm ten pozwala na dystrybucję jednego pliku, który zachowuje się jak instalator, ale nie wymaga uprawnień roota (instaluje się do folderu użytkownika).
