# Źródła launch video

Film: `../ogarniacz_launch_video.mp4` (66 s, 1920x1080, 60 fps).
Zrobiony skillem product-launch-motion (github.com/AbubakrChan/product-launch-motion):
cały film to jedna strona HTML z animacjami sprężynowymi (`film/index.html`, funkcja `seek(t)`),
renderowana klatka po klatce w Playwright z rozmyciem ruchu (4 podklatki na klatkę) i złożona w ffmpeg.
Muzyka i efekty są oryginalne, syntetyzowane lokalnie (`audio.py`, 120 BPM), bez licencjonowanych utworów.
Wszystkie teksty, daty i odpowiedzi Ogara pochodzą z działającej aplikacji.

Render od zera (wymaga node, ffmpeg, python3 z numpy):

```bash
npm install
npx playwright install chromium-headless-shell
python3 audio.py        # song.wav, sfx.wav, mix.wav
node render.mjs         # render/film.mp4
```

`render.mjs` ma na sztywno ścieżkę do Chromium z cache Playwright (`EXE`); przy innej wersji popraw ją.
Plan reżyserski scen: `PLAN.md`.
