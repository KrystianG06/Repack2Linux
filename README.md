# Repack2Linux (R2L)

Narzędzie automatyzujące instalację wideo-repacków z Windowsa na Linuxie przy użyciu Wine/Proton.
Zaprojektowane, aby maksymalnie uprościć proces dla graczy korzystających z Linuxa.

<p>
  <img alt="Wersja" src="https://img.shields.io/badge/version-v1.3.0-1f6feb">
  <img alt="Platforma" src="https://img.shields.io/badge/platform-Linux-0a0a0a">
  <img alt="Status" src="https://img.shields.io/badge/status-stable-1f883d">
  <img alt="Silnik" src="https://img.shields.io/badge/engine-Wine%20%2B%20Proton-8b5cf6">
</p>

## Czym jest R2L?
Repack2Linux to aplikacja desktopowa, która konwertuje instalatory gier z Windowsa na gotowe do użycia, samodzielne paczki linuxowe.
Aplikacja automatycznie skanuje i konfiguruje prefiksy Wine, eksportuje wersje przenośne (Portable) i generuje inteligentne skrypty startowe (`play_auto.sh`) z mechanizmem automatycznego odzyskiwania.

## Dlaczego warto?
- Automatyzacja całego procesu – od źródła do gotowej gry.
- Architektura "Portable-first" – izolacja zapisów gier i danych wewnątrz paczki.
- Inteligentne profile uczące się optymalnych ustawień (wymagania, wersje Protona).
- Solidny mechanizm startowy dla mniej zaawansowanych użytkowników.
- **NOWOŚĆ:** Automatyczna ekstrakcja wysokiej jakości ikon bezpośrednio z plików EXE gier.

## Zrzuty ekranu
| Fabryka (Factory) | Eksport | Instalator |
|---|---|---|
| ![Factory](docs/assets/Factory.png) | ![Export](docs/assets/export.png) | ![Installer](docs/assets/installer.png) |

## Kluczowe Funkcje
| Funkcja | Co zyskujesz |
|---|---|
| Inteligentne skanowanie prefiksów | Wykorzystuje i ocenia istniejące prefiksy Wine/Lutris/Steam, zamiast budować wszystko od zera |
| Wyuczone profile | Automatycznie zapamiętuje udane konfiguracje (biblioteki, wersje Protona) dla każdej gry |
| Eksport Portable | Tworzy paczkę z `play.sh`, `play_auto.sh` i `play_safe.sh` |
| Bezpieczny Fallback | Automatyczne przełączanie na tryb bezpieczny w przypadku wykrycia problemów z renderowaniem |
| Izolowane środowisko | Zapisy i dane gry zostają wewnątrz paczki (`./pfx`, `./r2p_userdata`) |
| Ujednolicony instalator `.sh` | Opcjonalny samorozpakowujący się instalator z integracją z pulpitem i menu |
| **Integracja Ikon** | Ikona gry pojawia się automatycznie w UI Fabryki i skrótach pulpitowych |

## Zachowanie przy uruchamianiu
- `play_auto.sh` to zalecany sposób uruchamiania.
- Jeśli standardowy start zawiedzie, tryb auto ponawia próbę w trybie bezpiecznym (Safe Mode).
- Sprawdzenia systemowe wyjaśniają brakujące zależności, zamiast kończyć się cichym błędem.

## Struktura Projektu
```text
src/
  main.rs                # Powłoka aplikacji, stan UI, orkiestracja
  engine.rs              # Pipeline produkcyjny
  installer/             # Eksport, skrypty startowe, generowanie instalatora (SFX)
  detector.rs            # Detekcja gier i ekstrakcja ikon (Pelite + Image)
  database.rs            # SQLite + wyuczone profile JSON
  community_sync.rs      # Opcjonalna synchronizacja z GitHub
  ui/                    # Interfejs Iced
```

## Szybki Start
```bash
git clone https://github.com/KrystianG06/Repack2Linux.git
cd Repack2Linux
cargo run --bin repack2proton-rs
```

## Budowanie wersji stabilnej (Release)
```bash
chmod +x build_release.sh
./build_release.sh
```

Tworzy:
- `dist/Repack2Linux-v1.3.0-<target>.tar.gz`
- `dist/Repack2Linux-v1.3.0-<target>.sha256`

## Typowy Workflow
1. Wybierz folder źródłowy z grą (instalator lub wypakowana gra).
2. R2L wykryje i zaproponuje najlepszy profil/ustawienia.
3. Uruchom produkcję (testowe uruchomienie bezpośrednio z aplikacji).
4. Eksportuj gotową paczkę Portable (lub instalator SFX).
5. Uruchamiaj grę za pomocą `play_auto.sh`.

## Dokumentacja (Pozostałe pliki)
- Raport postępu: [`PROGRESS.md`](./PROGRESS.md)
- Przegląd projektu: [`PROJECT_OVERVIEW.md`](./PROJECT_OVERVIEW.md)
- Teksty promocyjne: [`LANDING_COPY.md`](./LANDING_COPY.md)

## FAQ
**Gdzie są zapisy gier?**  
Wewnątrz paczki: `./r2p_userdata` (zazwyczaj podfolder `Local`).

**Który skrypt uruchamiać?**  
Zawsze zalecamy `./play_auto.sh`.

**Jak dodać skrót do gry?**  
W zakładce `Konfiguracja` użyj przycisku `DODAJ SKRÓT`.

**Co jeśli gra ma czarny ekran?**  
Tryb auto sam spróbuje trybu bezpiecznego. Możesz też ręcznie uruchomić `./play_safe.sh`.

## Licencja
Projekt obecnie wewnętrzny. Wybór licencji publicznej planowany przed oficjalnym wydaniem.
