# Ogarniacz 🐕

**Planner zadań dla studenta, który nie tylko trzyma listę, ale mówi, co zrobić teraz.**
Zadanie eliminacyjne BEST Coding Marathon 2026, kategoria SmartApp. Napisane w **Rust** (axum).

Student ma kolokwia, projekty, pracę i pranie. Zwykła lista pokazuje mu 20 rzeczy naraz i tyle.
Ogarniacz liczy, co jest naprawdę pilne, rozkłada pracę na tydzień tak, żeby nic nie wybuchło
jednego dnia, a asystent **Ogar** (pies tropiący terminy) rozumie polecenia pisane jak do kumpla:
„kolokwium z fizyki w czwartek pilne”.

## Jak uruchomić

Wymagany Rust (stabilny, `rustup` z https://rustup.rs).

```bash
cd 1_APLIKACJA
cargo run --release
```

Otwórz http://localhost:3000. Front jest wkompilowany w binarkę, nie trzeba node ani npm.

Opcje (zmienne środowiskowe):

| zmienna | domyślnie | po co |
|---|---|---|
| `PORT` | `3000` | port serwera |
| `OGARNIACZ_PLIK` | `dane/zadania.json` | gdzie aplikacja trzyma zadania (autozapis po każdej zmianie) |
| `OGARNIACZ_DZIS` | dzisiejsza data | „udawana” data dla demo, np. `2026-11-22`, bo przykładowe zadania mają terminy z listopada 2026 |

Na pierwszym starcie wczytuje się 20 przykładowych zadań z `1_APLIKACJA/przyklady/zadania.json`.

Testy: `cargo test` (format pliku, daty po polsku, pilność, plan tygodnia, asystent).

## Co działa

Wymagania z zadania, wszystkie:

- lista zadań (widoki: Teraz, Na dziś, Po terminie, Wszystkie, Zrobione, filtr po tagach, wyszukiwarka, sortowanie),
- dodawanie zadania (formularz albo jednym zdaniem przez Ogara),
- edycja (klik w zadanie),
- wykreślanie jako zrobione i cofanie (kółko przy zadaniu, toast „Cofnij”),
- usuwanie (z możliwością cofnięcia przez kilka sekund),
- **wczytanie z pliku JSON** (przycisk „Wczytaj JSON”, zastępuje listę),
- **zapis do pliku JSON** (przycisk „Zapisz JSON”) w dokładnie tym samym formacie:
  `{ "wersja": 1, "zadania": [ { id, tytul, opis, termin, priorytet, wykonane, tagi } ] }`.
  `termin` może być `null`, `opis` pusty, `tagi` pustą listą, `priorytet` to `niski` / `sredni` / `wysoki`.
  Test `model::testy` sprawdza, że po wczytaniu i zapisie plik ma te same klucze i wartości,
  a ręczny test (wczytaj przykłady, zapisz) daje plik identyczny z wejściowym.

## Co dodaliśmy od siebie i dlaczego

1. **„Teraz”: wynik pilności zamiast samej listy.** Każde zadanie dostaje punkty: priorytet × bliskość
   terminu, zaległe rosną (z sufitem, żeby stare drobiazgi nie zjadły listy). Na górze karta
   „Zacznij od tego” z uzasadnieniem po ludzku („termin blisko (wtorek), priorytet wysoki”).
   Dlaczego: student nie potrzebuje widzieć wszystkiego, potrzebuje wiedzieć, od czego zacząć.

2. **Plan tygodnia, który rozkłada pracę PRZED terminem.** Zadanie o wysokim priorytecie ląduje
   2 dni przed terminem, średnie dzień wcześniej, maksymalnie 3 rzeczy dziennie, nadmiar idzie
   na wcześniejsze wolne dni. Dlaczego: kolokwium we wtorek trzeba robić w niedzielę, nie we wtorek.

3. **Ostrzeżenia o kumulacji.** „Wtorek 24.11 kumulacja: 2 terminy naraz. Zacznij kolokwium wcześniej.”
   Dlaczego: najgorsze tygodnie sesji widać zawsze dopiero, jak już jest za późno.

4. **Ogar: asystent AI (atrapa) w czacie obok listy.** Rozumie polecenia po polsku, bez sztywnej składni:
   - „dodaj kolokwium z analizy w piątek #matma” → tytuł, termin (piątek = konkretna data), priorytet
     (kolokwium = wysoki), tagi (#matma + rozpoznane #studia),
   - daty mówione: dziś, jutro, pojutrze, w środę, za 3 dni, za tydzień, do końca tygodnia, 05.12, 10 grudnia,
   - „co teraz?”, „co na dziś?”, „co po terminie?”, „jak mi idzie?”, „zaplanuj mi tydzień”,
   - „rozbij projekt na kroki” → proponuje 4–5 kroków z terminami rozłożonymi do deadline’u i po „tak” dodaje je jako zadania,
   - „zrobione pranie”, „przesuń czynsz na piątek”, „usuń siłownię” (z potwierdzeniem),
   - **tryb ratunkowy**: na „nie dam rady” / „stres” zawęża świat do 3 najpilniejszych rzeczy.

   Pokazuje też „tok myślenia” (co rozpoznał: intencja, data, priorytet), żeby było widać, że nie zgaduje.
   Dlaczego: student zapisuje zadania w biegu, między zajęciami. Jedno zdanie zamiast formularza z pięcioma polami.

   **Uczciwie:** to atrapa, nie model językowy. Działa na regułach w Ruście (`src/asystent.rs`), więc
   działa offline, natychmiast i za darmo. Ma jednak docelowy interfejs: trait `Mozg`. Podpięcie
   prawdziwego LLM (z narzędziami „dodaj / odhacz / zaplanuj”) to nowa implementacja tego traitu,
   reszta aplikacji się nie zmienia.

5. **Szybkie dodawanie zdaniem** w pasku nad listą: to samo parsowanie co Ogar.

6. **Tożsamość: Ogar.** Ogar polski to pies gończy, który tropi. Nasz tropi terminy, zanim one dopadną ciebie.
   Stąd nazwa aplikacji (ogarnąć = dać radę ze wszystkim) i maskotka.

Drobniejsze: statystyki (postęp, tagi, wykres terminów na 14 dni), tryb ciemny, wersja mobilna,
skróty klawiszowe (`n` nowe zadanie, `/` czat), autozapis na dysk.

## Czego nie zdążyliśmy

- Prawdziwy LLM za traitem `Mozg` (np. Claude z narzędziami). Interfejs jest gotowy, implementacji nie ma.
- Historia wykonania (kiedy zadanie odhaczono): format pliku z zadania nie ma takiego pola,
  a nie chcieliśmy go zmieniać. Bez tego nie liczymy serii dni ani tempa pracy.
- Powiadomienia (np. push „jutro kolokwium”).
- Import zastępuje całą listę; opcja dołączenia zadań z drugiego pliku byłaby wygodniejsza.

## Jak to jest zbudowane

```
1_APLIKACJA/
  src/main.rs      serwer axum, API REST, zapis na dysk
  src/model.rs     format pliku zadania.json (serde), test zgodności formatu
  src/smart.rs     pilność, plan tygodnia, statystyki, ostrzeżenia
  src/asystent.rs  Ogar: trait Mozg + implementacja regułowa (intencje, daty po polsku)
  static/          front: HTML + CSS + czysty JS (bez frameworków), wkompilowany w binarkę
  przyklady/       20 przykładowych zadań
2_FILM_PROMOCYJNY/ materiały do 2-minutowej prezentacji
```

API: `GET/POST /api/zadania`, `PUT/DELETE /api/zadania/{id}`, `PATCH /api/zadania/{id}/przelacz`,
`POST /api/import`, `GET /api/eksport`, `GET /api/smart`, `POST /api/asystent`.
