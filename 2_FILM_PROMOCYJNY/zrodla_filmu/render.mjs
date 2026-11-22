// Playwright subframes (4 per 60fps frame) -> ffmpeg tmix motion blur, chunked 4 s, N workers.
import { chromium } from 'playwright';
import fs from 'fs'; import path from 'path'; import { execFileSync } from 'child_process';
const EXE = process.env.HOME + '/Library/Caches/ms-playwright/chromium_headless_shell-1243/chrome-headless-shell-mac-arm64/chrome-headless-shell';
const FPS = 60, SUB = 4, CH = 4, WORKERS = +(process.env.WORKERS || 4);
const DUR = 66, nChunks = Math.ceil(DUR / CH);
fs.mkdirSync('render/seg', { recursive: true });
const subT = i => Math.floor(i / SUB) / FPS + ((i % SUB) - 1.5) / (FPS * SUB);
const queue = [...Array(nChunks).keys()].filter(c => !fs.existsSync(`render/seg/c${String(c).padStart(3, '0')}.mp4`));
async function worker(id) {
  let b = await chromium.launch({ executablePath: EXE });
  let p = null;
  const fresh = async () => { if (p) await p.close().catch(() => {}); p = await b.newPage({ viewport: { width: 1920, height: 1080 } }); await p.goto('file://' + path.resolve('film/index.html')); await p.waitForFunction(() => window.READY); };
  await fresh();
  while (queue.length) {
    const c = queue.shift(); const dir = `render/f${c}`; fs.mkdirSync(dir, { recursive: true });
    const i0 = c * CH * FPS * SUB, i1 = Math.min((c + 1) * CH, DUR) * FPS * SUB;
    for (let i = i0; i < i1; i++) {
      const t = Math.max(0, subT(i));
      for (let a = 0; a < 3; a++) { try { await p.evaluate(t => seek(t), t); await p.screenshot({ path: `${dir}/s${String(i - i0).padStart(5, '0')}.jpg`, type: 'jpeg', quality: 93 }); break; } catch (e) { if (!b.isConnected()) b = await chromium.launch({ executablePath: EXE }); p = null; await fresh(); } }
    }
    execFileSync('ffmpeg', ['-loglevel', 'error', '-y', '-framerate', String(FPS * SUB), '-i', `${dir}/s%05d.jpg`, '-vf', `tmix=frames=4:weights='1 1 1 1',select='eq(mod(n\\,4)\\,3)',setpts=N/(${FPS}*TB),scale=out_color_matrix=bt709:out_range=tv,format=yuv420p`, '-r', String(FPS), '-c:v', 'libx264', '-preset', 'medium', '-crf', '16', '-tune', 'film', '-colorspace', 'bt709', '-color_primaries', 'bt709', '-color_trc', 'bt709', `render/seg/c${String(c).padStart(3, '0')}.mp4`]);
    fs.rmSync(dir, { recursive: true }); console.log(`w${id} chunk ${c} done ${new Date().toISOString().slice(11, 19)}`);
  }
  process.stdout.write(''); await b.close().catch(() => {});
}
await Promise.all([...Array(WORKERS).keys()].map(worker));
const list = fs.readdirSync('render/seg').filter(f => f.endsWith('.mp4')).sort().map(f => `file 'seg/${f}'`).join('\n');
fs.writeFileSync('render/list.txt', list);
execFileSync('ffmpeg', ['-loglevel', 'error', '-y', '-f', 'concat', '-safe', '0', '-i', 'render/list.txt', '-i', 'mix.wav', '-map', '0:v', '-map', '1:a', '-c:v', 'copy', '-c:a', 'aac', '-b:a', '256k', '-shortest', '-movflags', '+faststart', 'render/film.mp4']);
console.log('DONE'); process.exit(0);
