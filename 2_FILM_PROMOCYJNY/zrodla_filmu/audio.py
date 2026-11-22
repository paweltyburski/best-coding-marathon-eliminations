# Original score + synthesized UI SFX for the Ogarniacz film. Pure numpy, no samples, no licences needed.
# python3 audio.py  -> song.wav, sfx.wav, mix.wav (48 kHz stereo)
import json, wave
import numpy as np

SR = 48000; BPM = 120; BEAT = 60 / BPM; DUR = 66.0
N = int(SR * DUR)
rng = np.random.default_rng(7)
def tt(d): return np.arange(int(SR * d)) / SR
def env(n, a=0.002, d=0.2):
    t = np.arange(n) / SR
    return np.minimum(1, t / max(a, 1e-4)) * np.exp(-t / d)
def lp(x, a):  # one-pole lowpass, a in (0,1)
    y = np.empty_like(x); s = 0.0
    for i in range(len(x)): s += a * (x[i] - s); y[i] = s
    return y
def lp_fast(x, cutoff):  # FFT brickwall-ish lowpass (fast)
    X = np.fft.rfft(x); f = np.fft.rfftfreq(len(x), 1 / SR)
    X *= 1 / (1 + (f / cutoff) ** 4); return np.fft.irfft(X, len(x))
def hp_fast(x, cutoff):
    X = np.fft.rfft(x); f = np.fft.rfftfreq(len(x), 1 / SR)
    X *= 1 / (1 + (cutoff / np.maximum(f, 1)) ** 4); return np.fft.irfft(X, len(x))
def bp_fast(x, lo, hi): return hp_fast(lp_fast(x, hi), lo)
def mtof(m): return 440 * 2 ** ((m - 69) / 12)
def add(buf, x, t, g=1.0):
    i = int(round(t * SR))
    if i < 0: x = x[-i:]; i = 0
    j = min(len(buf), i + len(x))
    if j > i: buf[i:j] += g * x[:j - i]

# ── instruments ──
def kick(g=1.0):
    t = tt(0.45); f = 45 + 110 * np.exp(-t / 0.035)
    ph = 2 * np.pi * np.cumsum(f) / SR
    x = np.sin(ph) * np.exp(-t / 0.16) + 0.25 * np.sin(ph) * np.exp(-t / 0.02)
    cl = rng.standard_normal(len(t)) * np.exp(-t / 0.003) * 0.3
    return g * np.tanh(1.6 * (x + cl))
def clap():
    t = tt(0.3); n = rng.standard_normal(len(t))
    e = sum(np.exp(-np.maximum(t - o, 0) / 0.012) * (t >= o) for o in (0, 0.011, 0.022)) + 0.6 * np.exp(-t / 0.09)
    return bp_fast(n * e, 900, 5000) * 0.55
def hat(d=0.04, g=0.18):
    t = tt(d * 3); return hp_fast(rng.standard_normal(len(t)), 7000) * np.exp(-t / d) * g
def snare():
    t = tt(0.22); n = bp_fast(rng.standard_normal(len(t)), 1200, 7000) * np.exp(-t / 0.07)
    return 0.5 * n + 0.3 * np.sin(2 * np.pi * 190 * t) * np.exp(-t / 0.04)
def saw(f, d):
    t = tt(d); x = np.zeros_like(t)
    for det in (-0.08, 0, 0.08):
        x += 2 * ((t * f * 2 ** (det / 12)) % 1) - 1
    return x / 3
def pluck(m, d=0.35, g=0.3, bright=4000):
    t = tt(d + 0.3); f = mtof(m)
    x = (2 * ((t * f) % 1) - 1) * 0.6 + np.sin(2 * np.pi * f * t) * 0.6
    return lp_fast(x * env(len(t), 0.002, d * 0.45), bright) * g
def bass(m, d, g=0.42):
    t = tt(d); f = mtof(m)
    x = np.sin(2 * np.pi * f * t) + 0.35 * np.sign(np.sin(2 * np.pi * f * t)) * 0.5
    e = np.minimum(1, t / 0.005) * np.minimum(1, (d - t) / 0.02)
    return lp_fast(x * e, 600) * g
def pad(ms, d, g=0.12, cut=1800):
    x = sum(saw(mtof(m), d) for m in ms)
    t = tt(d); e = np.minimum(1, t / 0.4) * np.minimum(1, (d - t) / 0.5)
    return lp_fast(x * e, cut) * g

song = np.zeros(N); padbus = np.zeros(N); duck = np.ones(N)
# chords per bar (A minor, warm): Am F C G
CH = [[57, 60, 64], [53, 57, 60], [48, 52, 55], [55, 59, 62]]
BASSN = [45, 41, 36, 43]
bar = lambda k: k * 2 * BEAT * 2  # bar k start (2 s)
nbars = 33
def sec(t):
    if t < 4: return 'intro'
    if t < 10: return 'groove'
    if t < 14: return 'build'
    if t < 22: return 'drop'
    if t < 34: return 'verse'
    if t < 42: return 'lift'
    if t < 48: return 'verse'
    if t < 50: return 'down'
    if t < 56: return 'break'
    if t < 60: return 'drop2'
    return 'end'

for k in range(nbars):
    t0 = k * 2.0; c = CH[k % 4]; b = BASSN[k % 4]; s = sec(t0)
    if t0 >= 60: break
    # pad everywhere (brighter in drops)
    cut = {'intro': 900, 'groove': 1200, 'build': 1500, 'drop': 2600, 'verse': 1500, 'lift': 2400, 'down': 900, 'break': 1100, 'drop2': 2800}[s]
    add(padbus, pad([m + 12 for m in c], 2.05, 0.10 if s != 'break' else 0.13, cut), t0)
    for q in range(4):
        tb = t0 + q * BEAT
        drums = s in ('groove', 'drop', 'verse', 'lift', 'drop2') or (s == 'build' and tb < 13.0) or (s == 'intro' and q == 0) or (s == 'down' and tb < 49)
        if drums:
            add(song, kick(1.0 if s in ('drop', 'lift', 'drop2') else 0.85), tb)
            d = np.ones(int(0.32 * SR)); tt_ = np.arange(len(d)) / SR; d -= 0.65 * np.exp(-tt_ / 0.12)
            i = int(tb * SR); duck[i:i + len(d)] = np.minimum(duck[i:i + len(d)], d[:max(0, min(len(d), N - i))])
        if s in ('drop', 'verse', 'lift', 'drop2') and q in (1, 3): add(song, clap(), tb)
        if s in ('groove', 'verse'): add(song, hat(0.03, 0.12), tb + BEAT / 2)
        if s in ('drop', 'lift', 'drop2'):
            for h in range(4): add(song, hat(0.025, 0.10 if h % 2 else 0.14), tb + h * BEAT / 4)
        # bass: offbeat 8ths in groove sections
        if s in ('groove', 'drop', 'verse', 'lift', 'drop2', 'build') and not (s == 'build' and tb >= 13):
            add(song, bass(b, BEAT / 2 * 0.9), tb + BEAT / 2)
            if s in ('drop', 'lift', 'drop2'): add(song, bass(b + 12, BEAT / 4 * 0.8, 0.25), tb + BEAT * 0.75)
    # arp
    if s in ('drop', 'lift', 'drop2', 'intro', 'break', 'groove'):
        notes = [c[0] + 12, c[1] + 12, c[2] + 12, c[1] + 24, c[2] + 12, c[0] + 24, c[1] + 12, c[2] + 24]
        step = BEAT / 4 if s in ('drop', 'lift', 'drop2') else BEAT / 2
        n = int(2.0 / step)
        for i in range(n):
            g = 0.16 if s in ('drop', 'lift', 'drop2') else 0.12
            if s == 'groove' and i % 2: continue
            add(song, pluck(notes[i % 8], 0.18 if step < 0.2 else 0.4, g, 3500 if s != 'break' else 2200), t0 + i * step)

# hook stabs 0, 1.5, 3.0
for th in (0.0, 1.5, 3.0):
    for m in CH[0]: add(song, pluck(m + 12, 0.5, 0.22, 5000), th)
    add(song, kick(0.9), th)
# build: snare roll accelerating 10-14 + riser
for t0, t1, div in ((10, 11, 1), (11, 12, 2), (12, 13, 4), (13, 13.75, 8)):
    st = BEAT / div; n = int(round((t1 - t0) / st))
    for i in range(n): add(song, snare(), t0 + i * st, 0.35 + 0.5 * (t0 - 10 + i * st / 1) / 4)
def riser(d, g=0.25, f0=400, f1=6000):
    t = tt(d); n = rng.standard_normal(len(t))
    out = np.zeros_like(t); seg = int(SR * 0.05)
    for i in range(0, len(t), seg):
        fc = f0 * (f1 / f0) ** (i / len(t)); out[i:i + seg] = bp_fast(n[i:i + seg + 0], fc * 0.7, fc * 1.4)[:len(out[i:i + seg])]
    return out * (t / d) ** 2 * g
add(song, riser(3.9, 0.3), 10.0)
for t0, t1, div in ((54, 55, 2), (55, 55.75, 4)):
    st = BEAT / div; n = int(round((t1 - t0) / st))
    for i in range(n): add(song, snare(), t0 + i * st, 0.3 + 0.15 * i / n)
add(song, riser(1.9, 0.22), 54.0)
# basics slams accelerate (kick on 16ths 59-60)
for i, tb in enumerate([59.0, 59.25, 59.5, 59.75]): add(song, kick(0.9), tb)
# ending: big hit at 60 and long tail
endc = [45, 57, 60, 64, 69, 72]
t = tt(6.0)
tail = sum(saw(mtof(m), 6.0) for m in endc) * np.exp(-t / 1.8) * np.minimum(1, t / 0.01)
add(song, lp_fast(tail, 2500) * 0.13, 60.0)
add(song, kick(1.2), 60.0); add(song, clap(), 60.0, 1.2)
for i, m in enumerate([69, 72, 76, 81]): add(song, pluck(m, 0.6, 0.16, 4000), 61.0 + i * 0.25)
for i, m in enumerate([76, 72, 69]): add(song, pluck(m, 0.8, 0.12, 3000), 63.0 + i * 0.5)
# simple stereo-ish delay/reverb on pads & plucks
song = song + padbus * duck
def verb(x, g=0.25):
    y = x.copy()
    for dly, a in ((0.031, .5), (0.047, .4), (0.071, .33), (0.113, .25), (0.167, .2), (0.251, .14)):
        i = int(dly * SR); y[i:] += a * g * x[:-i]
    return y
L = verb(song, 0.28); R = verb(np.roll(song, int(0.009 * SR)), 0.28)
song_st = np.stack([L, R], 1)
fade = np.ones(N); fi = int(64.8 * SR); fade[fi:] = np.linspace(1, 0, N - fi) ** 2
song_st *= fade[:, None]

# ── beat grid analysis (numpy): verify tempo and measure kick-peak offset ──
mono = song_st.mean(1)
hop = 480; fr = 2048
frames = np.lib.stride_tricks.sliding_window_view(np.pad(mono, (0, fr)), fr)[::hop][: N // hop]
spec = np.abs(np.fft.rfft(frames * np.hanning(fr), axis=1))
kb = spec[:, 1:12].sum(1)  # ~23-250 Hz kick band
flux = np.maximum(0, np.diff(np.log1p(spec), axis=0)).sum(1)
ac = np.correlate(flux - flux.mean(), flux - flux.mean(), 'full')[len(flux) - 1:]
lags = np.arange(len(ac)) * hop / SR
rngl = (lags > 0.3) & (lags < 1.0)
beat_est = lags[rngl][np.argmax(ac[rngl])]
print(f'tempo estimate: {60 / beat_est:.1f} BPM (target {BPM})')
# kick peak offset relative to grid in the drop
offs = []
for k in range(28, 44):
    tb = k * BEAT; i0 = int(tb * SR); seg = np.abs(mono[i0:i0 + int(0.05 * SR)])
    offs.append(np.argmax(seg) / SR)
PEAK_OFF = float(np.median(offs)); print(f'kick peak offset {PEAK_OFF * 1000:.1f} ms')

# ── SFX, each placed by its own measured peak on the cue ──
def s_click(g):
    t = tt(0.08); x = np.sin(2 * np.pi * 2400 * t) * np.exp(-t / 0.004) + hp_fast(rng.standard_normal(len(t)), 3000) * np.exp(-t / 0.002) * .6
    y = np.zeros(int(0.14 * SR)); y[:len(x)] += x; y[int(0.06 * SR):int(0.06 * SR) + len(x)] += 0.45 * x  # press + release
    return y * 0.32 * g
def s_key(g):
    t = tt(0.05); f = 1800 + rng.uniform(-300, 300)
    return (bp_fast(rng.standard_normal(len(t)), f, f * 2.5) * np.exp(-t / 0.006) + 0.3 * np.sin(2 * np.pi * f / 2 * t) * np.exp(-t / 0.008)) * 0.28 * g
def s_tick(g):
    t = tt(0.05); return np.sin(2 * np.pi * 3200 * t) * np.exp(-t / 0.006) * 0.16 * g
def s_pop(g):
    t = tt(0.16); f = 300 + 900 * np.exp(-t / 0.02)
    return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.05) * 0.36 * g
def s_blip(g):
    t = tt(0.12); m = [69, 72, 76, 79, 81, 84][int(g * 10) % 6]
    return np.sin(2 * np.pi * mtof(m + 12) * t) * env(len(t), 0.002, 0.04) * 0.14
def s_chime(g):
    t = tt(0.9); x = sum(np.sin(2 * np.pi * mtof(m) * t) * a for m, a in ((84, 1), (88, .6), (91, .4)))
    return x * env(len(t), 0.003, 0.25) * 0.12 * g
def s_note(g):
    t = tt(0.8); m = 72 + int((g - .8) * 30); return np.sin(2 * np.pi * mtof(m) * t) * env(len(t), 0.005, 0.3) * 0.16
def s_whoosh(g, d=0.45):
    t = tt(d); n = rng.standard_normal(len(t)); e = np.sin(np.pi * t / d) ** 2
    x = bp_fast(n, 500, 3500) * e
    return x / (np.abs(x).max() + 1e-9) * 0.2 * g
def s_slam(g):
    t = tt(1.2); x = np.sin(2 * np.pi * np.cumsum(40 + 80 * np.exp(-t / 0.05)) / SR) * np.exp(-t / 0.35)
    x += bp_fast(rng.standard_normal(len(t)), 200, 4000) * np.exp(-t / 0.08) * 0.5
    return np.tanh(x * 1.5) * 0.5 * g
def s_thump(g):
    t = tt(0.3); return np.sin(2 * np.pi * np.cumsum(60 + 100 * np.exp(-t / 0.03)) / SR) * np.exp(-t / 0.1) * 0.4 * g
def s_pen(g):
    t = tt(0.6); return bp_fast(rng.standard_normal(len(t)), 2000, 6000) * np.sin(np.pi * t / 0.6) * 0.08 * g
def s_alert(g):
    t = tt(0.5); x = np.sin(2 * np.pi * mtof(76) * t) * env(len(t), .002, .08)
    y = np.zeros(int(0.7 * SR)); y[:len(x)] += x; i = int(0.12 * SR); y[i:i + len(x)] += np.sin(2 * np.pi * mtof(72) * t) * env(len(t), .002, .12)
    return y * 0.14 * g
def s_swell(g):
    t = tt(1.4); x = sum(np.sin(2 * np.pi * mtof(m) * t) for m in (69, 72, 76)) * np.sin(np.pi * t / 1.4) ** 2
    return x * 0.06 * g
def s_send(g):
    a = s_whoosh(g * .8, 0.25); b = s_pop(g * .6); y = np.zeros(max(len(a), len(b))); y[:len(a)] += a; y[:len(b)] += b; return y
SYN = dict(click=s_click, key=s_key, tick=s_tick, pop=s_pop, blip=s_blip, chime=s_chime, note=s_note, whoosh=s_whoosh,
           swoosh=lambda g: s_whoosh(g, 0.8), slam=s_slam, thump=s_thump, pen=s_pen, alert=s_alert, swell=s_swell, send=s_send,
           kick=lambda g: kick(g) * 0.35)
cues = json.load(open('events.json'))
sfx = np.zeros(N)
for c in cues:
    x = SYN[c['type']](c['g'])
    pk = np.argmax(np.abs(x)) / SR
    if c['type'] in ('whoosh', 'swoosh', 'swell', 'pen'): pk = len(x) / SR * 0.55  # peak of a swell = arrival
    add(sfx, x, c['t'] + PEAK_OFF - pk)
sfx_st = np.stack([sfx, sfx], 1)
mix = song_st * 0.5 + sfx_st * 0.62
mix *= 0.93 / np.abs(mix).max()
def wr(name, x):
    x16 = (np.clip(x, -1, 1) * 32767).astype(np.int16)
    with wave.open(name, 'wb') as w: w.setnchannels(2); w.setsampwidth(2); w.setframerate(SR); w.writeframes(x16.tobytes())
wr('song.wav', song_st / np.abs(song_st).max() * 0.9); wr('mix.wav', mix)
print('cues', len(cues), 'done')
