# Wersja C: product-launch-motion. Plan reżyserski

Koncepcja: jeden kształt, bez cięć. Kształtem jest zadanie „Kolokwium z algebry liniowej” (hook: „Kolokwium we wtorek.”),
które staje się kartą „Zacznij od tego”, czatem Ogara, kartą „Kolokwium z fizyki”, kafelkiem w planie tygodnia,
kartą trybu ratunkowego, przyciskami podstaw i na końcu pomarańczowym kwadratem logo Ogara.

Muzyka: oryginalna, syntetyzowana lokalnie w numpy (audio.py), 120 BPM, A-moll, 33 takty = 66 s.
Intro 0-4, groove 4-10, build 10-14 (werbel przyspiesza), DROP 14 (karta „Zacznij od tego”), groove 22-42,
podbicie 34-42 (plan tygodnia), breakdown 50-56 (tryb ratunkowy), drugi drop 56 (podstawy, 16-tki), uderzenie 60 (logo), ogon do 66.

Sceny (s):
- 0-6 hook: trzy pigułki na beat, odjazd kamery do ściany 20 zadań
- 6-14 problem: „20 rzeczy naraz”, pilność wskakuje na wiersze, sortowanie, najazd na wiersz nr 1
- 14-22 DROP: karta „Zacznij od tego” + licznik pilności 126 + priorytet × termin
- 22-31 czat Ogara: „dodaj kolokwium z fizyki w czwartek #fizyka”, tok myślenia, „Dodane!”
- 31-41 „zaplanuj mi tydzień”: plan 7 dni, kolokwium we wtorek (2 dni przed terminem), ostrzeżenie o kumulacji
- 41-50 „Rozbij kolokwium z fizyki na kroki”: 5 kroków, „Tak, dodaj”
- 50-56 „nie dam rady”: tryb ratunkowy, 3 rzeczy
- 56-60 podstawy na beat: Dodaj, Edytuj, Odhacz, Usuń, Wczytaj JSON, Zapisz JSON
- 60-66 logo, „Ogarniacz.”, „Ogarnij semestr.”, Rust · open source, repo

Dane: wszystkie tytuły, terminy, pilności, plan tygodnia, ostrzeżenia i odpowiedzi Ogara pochodzą z działającej aplikacji
(port 3103, OGARNIACZ_DZIS=2026-11-22, przykładowe dane + dodane kolokwium z fizyki).

DECISIONs:
- DECISION: jasne tło #F6F3EF (jak aplikacja), akcent #F2683A, jeden krój systemowy jak w aplikacji.
- DECISION: kolejność: dodaj → plan tygodnia → rozbij → ratunek (plan pokazuje dodane kolokwium we wtorek, 2 dni przed czwartkiem; ratunek po dodaniu kroków zwraca 3 kroki fizyki, prawdziwa odpowiedź aplikacji).
- DECISION: bez hasła „max 3 dziennie”, bo realny plan ma 4 rzeczy w niektóre dni.
- DECISION: muzyka syntetyzowana lokalnie (brak licencjonowanego utworu), bez lektora.

## Wykonanie (22.11.2026)
- skill-src/: sklonowany skill (v2, root SKILL.md + references). Skill nie dostarcza skryptów v2 (referencyjna implementacja
  jest w ~/code/cheerful-motion u autora, niedostępna), więc film.js/render/audio napisane od zera według references/technique.md.
- film/index.html: cały film, czysta funkcja seek(t), sprężyny w formie zamkniętej, Track = suma sprężyn.
- snap.mjs: klatki kontrolne + eksport cue'ów do events.json. render.mjs: 4 subklatki na klatkę 60 fps, ffmpeg tmix, segmenty 4 s.
- audio.py: muzyka i SFX syntetyzowane w numpy; analiza: tempo 120.0 BPM, offset szczytu stopy 21.5 ms; miks -17.5 LUFS, -0.6 dBTP.
- Dźwięk tylko zmierzony, nie odsłuchany.
