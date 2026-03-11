# R2L (Repack2Linux) - Przegląd Projektu

## Executive Summary
R2P to aplikacja desktopowa dla Linuxa, która automatyzuje budowę przenośnych paczek gier Windows (prefix + launcher + opcjonalny installer `.sh`).  
Jej celem jest zamiana ręcznej konfiguracji Wine/Proton na powtarzalny proces "zbuduj raz, uruchamiaj wszędzie".

## Problem vs Rozwiązanie

| Problem gracza na Linuxie | Jak rozwiązuje to R2P |
|---|---|
| Ręczna konfiguracja Wine, winetricks i bibliotek trwa długo | Automatyczny pipeline produkcji prefixu i presetów |
| Gra działa na jednym PC, na drugim trzeba robić wszystko od nowa | Eksport do paczki portable i/lub instalatora `.sh` |
| Czarne ekrany po migracji gry między komputerami | Safe Mode (`WineD3D`) + fallback renderera |
| Prefix zawiera ślady hosta i sztywne ścieżki | Relatywizacja i sanityzacja hive'ów `.reg` |
| Brak wspólnej wiedzy dla kilku osób | Auto-learning profili + opcjonalny GitHub community sync |

## Głęboka Analiza Funkcji

### 1. Sanityzacja i relatywizacja prefixu
Po produkcji prefixu R2P:
- normalizuje foldery użytkownika wewnątrz `drive_c/users`,
- stosuje poprawki rejestru (shell folders, input),
- czyści ślady hosta (user, hostname, ścieżki absolutne),
- filtruje wpisy monitor/GPU metadata (np. EDID), które mogą psuć przenośność.

**Efekt:** większa szansa uruchomienia paczki na innym PC bez konfliktów środowiska.

### 2. Inteligentny Prefix Scanning
R2P skanuje istniejące prefixy (Wine/Lutris/Steam), ocenia je i wybiera najlepszą bazę:
- wykrywanie architektury (`win32`/`win64`),
- scoring jakości prefixu,
- fallback na nowy prefix, jeśli kandydaty są słabe.

**Efekt:** oszczędność czasu i mniej ręcznej pracy w winetricks.

### 3. Safe Mode i fallback renderera
R2P wspiera:
- `play.sh --safe`,
- `play_safe.sh`,
- automatyczny fallback do `WineD3D` przy problemach środowiska Vulkan.

**Efekt:** stabilniejsze uruchamianie gier z problemami DXVK (np. "black screen").

### 4. Dependency Check w launcherze
`play.sh` sprawdza kluczowe zależności (`wine`, `wineserver`) i zwraca czytelny komunikat o brakach.

**Efekt:** mniej "cichych" błędów i szybsza diagnoza problemów użytkownika końcowego.

### 5. Auto-learning profili
Po udanej produkcji R2P zapisuje "nauczony" profil (prefix/proton/biblioteki) do JSON, aby kolejne buildy były automatycznie lepsze.

**Efekt:** narzędzie uczy się z praktyki i skraca konfigurację przy kolejnych grach.

### 6. Community Sync (GitHub)
R2P może synchronizować profile do wspólnego repozytorium:
- kolejka retry,
- odporność na chwilowe błędy push/pull,
- wspólna baza presetów dla wielu użytkowników.

**Efekt:** jeden spójny "knowledge base" dla całego zespołu.

### 7. Eksport i dystrybucja
Tryby eksportu:
- portable folder,
- installer `.sh`,
- inteligentna ekstrakcja ikon (PNG, brak artefaktów),
- helper tworzenia ikon/skrótów desktop/menu po instalacji.

**Efekt:** gotowy artefakt do uruchomienia bez ponownego konfigurowania, z profesjonalnym wyglądem w systemie.

## Workflow Produkcji Gry (End-to-End)

1. Użytkownik wskazuje folder gry/repacka.
2. R2P wykrywa EXE i analizuje wymagania.
3. Ładowany jest najlepszy profil (learned/cloud/heurystyka).
4. Prefix jest tworzony lub pobierany przez inteligentny scanning.
5. Instalowane są wymagane komponenty (DXVK/VC++/XAudio itp.).
6. Prefix przechodzi sanityzację i relatywizację.
7. Generowane są launchery (`play.sh`, `play_safe.sh`) oraz assets.
8. Gra jest uruchamiana testowo.
9. Użytkownik eksportuje portable i/lub installer `.sh`.
10. Profil jest zapisywany do bazy i opcjonalnie synchronizowany przez GitHub.

## Dlaczego to jest unikalne? (R2P vs Lutris/Bottles)

| Obszar | R2P | Lutris | Bottles |
|---|---|---|---|
| Główny cel | Produkcja artefaktu portable/offline | Launcher i orkiestracja gier | Zarządzanie butelkami Wine |
| Gotowy portable pakiet | Tak (core feature) | Ograniczone | Ograniczone |
| Installer `.sh` | Tak | Zwykle nie | Zwykle nie |
| Auto-learning profili | Tak | Ograniczone | Ograniczone |
| GitHub sync wiedzy | Tak | Nie natywnie | Nie natywnie |
| Relatywizacja pod migrację PC | Tak, projektowo kluczowa | Nie jako priorytet | Nie jako priorytet |
| Offline-first dystrybucja | Tak | Raczej nie | Raczej nie |

## Value Proposition
R2P nie jest tylko launcherem. To pipeline produkcyjny, który tworzy gotowe, przenośne artefakty gier i automatycznie zachowuje wiedzę z udanych konfiguracji.

