// Ogarniacz: front. Cała logika „smart” i asystent działają w Ruście (API),
// tu tylko rysujemy i wysyłamy polecenia.
"use strict";

const $ = (s) => document.querySelector(s);
const stan = { zadania: [], smart: null, widok: "teraz", tag: null, szukaj: "", sortuj: "smart", edytowane: null };

const NAZWY_WIDOKOW = {
  teraz: "Teraz", dzis: "Na dziś", zalegle: "Po terminie", wszystkie: "Wszystkie",
  zrobione: "Zrobione", tydzien: "Plan tygodnia", statystyki: "Statystyki",
};
const PRIO = { niski: 1, sredni: 2, wysoki: 3 };
const PRIO_NAZWA = { niski: "niski", sredni: "średni", wysoki: "wysoki" };

// ───────── API ─────────
async function api(sciezka, opcje = {}) {
  const r = await fetch(sciezka, {
    headers: opcje.body && typeof opcje.body === "string" && !opcje.raw ? { "Content-Type": "application/json" } : {},
    ...opcje,
  });
  if (!r.ok) {
    let msg = r.statusText;
    try { msg = (await r.json()).blad || msg; } catch (_) {}
    throw new Error(msg);
  }
  return r.status === 204 ? null : r.json();
}

async function odswiez() {
  const [plik, smart] = await Promise.all([api("/api/zadania"), api("/api/smart")]);
  stan.zadania = plik.zadania;
  stan.smart = smart;
  rysuj();
}

// ───────── daty ─────────
const dzisDate = () => (stan.smart ? stan.smart.dzis : new Date().toISOString().slice(0, 10));
function dniDo(termin) {
  if (!termin) return null;
  return Math.round((Date.parse(termin) - Date.parse(dzisDate())) / 86400000);
}
const DNI = ["niedziela", "poniedziałek", "wtorek", "środa", "czwartek", "piątek", "sobota"];
function opisTerminu(termin) {
  const d = dniDo(termin);
  if (d === null) return "bez terminu";
  if (d < -1) return `${-d} dni po terminie`;
  if (d === -1) return "wczoraj minął termin";
  if (d === 0) return "dziś";
  if (d === 1) return "jutro";
  if (d < 7) return DNI[new Date(termin).getDay()];
  return new Date(termin).toLocaleDateString("pl-PL", { day: "numeric", month: "short" });
}
function dodajDni(n) {
  const d = new Date(dzisDate());
  d.setDate(d.getDate() + n);
  return d.toISOString().slice(0, 10);
}

// ───────── rysowanie ─────────
function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}

function rysuj() {
  const s = stan.smart;
  const st = s.statystyki;
  $("#dzisiaj").textContent = new Date(s.dzis).toLocaleDateString("pl-PL", { weekday: "long", day: "numeric", month: "long" });
  $("#postep-pasek").style.width = st.procent + "%";
  $("#postep-tekst").textContent = st.procent + "%";
  $("#licz-teraz").textContent = s.ranking.length || "";
  $("#licz-dzis").textContent = st.na_dzis || "";
  $("#licz-zalegle").textContent = st.po_terminie || "";
  $("#licz-wszystkie").textContent = st.wszystkie;
  $("#licz-zrobione").textContent = st.wykonane || "";

  $("#ostrzezenia").innerHTML = s.ostrzezenia.map((o) => `<div class="ostrz">⚠️ <span>${esc(o)}</span></div>`).join("");

  // tagi
  const tagi = [...new Set(stan.zadania.flatMap((z) => z.tagi))].sort();
  $("#tagi-filtr").innerHTML = tagi
    .map((t) => `<button class="tag ${stan.tag === t ? "aktywny" : ""}" data-tag="${esc(t)}">#${esc(t)}</button>`)
    .join("");

  document.querySelectorAll("#widoki button").forEach((b) => b.classList.toggle("aktywny", b.dataset.widok === stan.widok));
  $("#widok-lista").hidden = ["tydzien", "statystyki"].includes(stan.widok);
  $("#widok-tydzien").hidden = stan.widok !== "tydzien";
  $("#widok-statystyki").hidden = stan.widok !== "statystyki";

  if (stan.widok === "tydzien") return rysujTydzien();
  if (stan.widok === "statystyki") return rysujStaty();
  rysujListe();
}

function filtruj() {
  const pkt = Object.fromEntries(stan.smart.ranking.map((r) => [r.id, r]));
  let z = stan.zadania.slice();
  switch (stan.widok) {
    case "teraz": z = z.filter((x) => !x.wykonane); break;
    case "dzis": z = z.filter((x) => !x.wykonane && dniDo(x.termin) !== null && dniDo(x.termin) <= 0); break;
    case "zalegle": z = z.filter((x) => !x.wykonane && dniDo(x.termin) !== null && dniDo(x.termin) < 0); break;
    case "zrobione": z = z.filter((x) => x.wykonane); break;
  }
  if (stan.tag) z = z.filter((x) => x.tagi.includes(stan.tag));
  if (stan.szukaj) {
    const q = stan.szukaj.toLowerCase();
    z = z.filter((x) => (x.tytul + " " + x.opis + " " + x.tagi.join(" ")).toLowerCase().includes(q));
  }
  const sort = {
    smart: (a, b) => (pkt[b.id]?.pilnosc || 0) - (pkt[a.id]?.pilnosc || 0) || a.wykonane - b.wykonane,
    termin: (a, b) => (a.termin || "9999").localeCompare(b.termin || "9999"),
    priorytet: (a, b) => PRIO[b.priorytet] - PRIO[a.priorytet],
    tytul: (a, b) => a.tytul.localeCompare(b.tytul, "pl"),
  }[stan.sortuj];
  z.sort(sort);
  return { z, pkt };
}

function rysujListe() {
  $("#tytul-widoku").textContent = NAZWY_WIDOKOW[stan.widok] + (stan.tag ? ` · #${stan.tag}` : "");
  const { z, pkt } = filtruj();

  const top = stan.smart.ranking[0];
  $("#hero").innerHTML =
    stan.widok === "teraz" && top && !stan.tag && !stan.szukaj
      ? `<div class="hero"><div><div class="etyk">⚡ Zacznij od tego</div><h3>${esc(top.tytul)}</h3>
         <p>Bo ${esc(top.dlaczego)}.</p></div>
         <button class="btn" data-zrob="${top.id}">✓ Zrobione</button></div>`
      : "";

  $("#lista").innerHTML = z
    .map((x) => {
      const d = dniDo(x.termin);
      const klasaTerminu = d === null ? "" : d < 0 && !x.wykonane ? "zalegle" : d === 0 ? "dzis" : "";
      const p = pkt[x.id];
      return `<li class="zad p-${x.priorytet} ${x.wykonane ? "zrobione" : ""}" data-id="${x.id}">
        <button class="ptaszek" data-akcja="przelacz" title="${x.wykonane ? "Cofnij" : "Oznacz jako zrobione"}">${x.wykonane ? "✓" : ""}</button>
        <div class="tresc" data-akcja="edytuj">
          <div class="tyt">${esc(x.tytul)}</div>
          ${x.opis ? `<div class="opis">${esc(x.opis)}</div>` : ""}
          <div class="meta">
            <span class="termin ${klasaTerminu}">${esc(opisTerminu(x.termin))}</span>
            ${x.tagi.map((t) => `<span class="tag" data-tag="${esc(t)}">#${esc(t)}</span>`).join("")}
            ${p && stan.sortuj === "smart" ? `<span class="pkt" title="${esc(p.dlaczego)}">pilność ${p.pilnosc}</span>` : ""}
          </div>
        </div>
        <div class="przyciski">
          <button class="ikona" data-akcja="edytuj" title="Edytuj">✏️</button>
          <button class="ikona" data-akcja="usun" title="Usuń">🗑</button>
        </div>
      </li>`;
    })
    .join("");
  $("#pusto").hidden = z.length > 0;
}

function rysujTydzien() {
  const byId = Object.fromEntries(stan.zadania.map((z) => [z.id, z]));
  $("#tydzien").innerHTML = stan.smart.plan
    .map((d) => {
      const ciezki = d.terminy >= 2 || d.obciazenie >= 7;
      return `<div class="dzien ${ciezki ? "ciezki" : ""}">
        <h4>${esc(d.etykieta)}</h4>
        <div class="ile">${d.zadania.length ? d.zadania.length + " do zrobienia" : d.terminy ? "zrobione wcześniej ✓" : "luz 😌"}${d.terminy ? ` · ${d.terminy} termin${d.terminy > 1 ? "y" : ""}` : ""}</div>
        ${d.zadania
          .map((s) => {
            const z = byId[s.id];
            return `<div class="kafel p-${z ? z.priorytet : "sredni"}" data-id="${s.id}" data-akcja="edytuj">${esc(s.tytul)}<small>termin: ${esc(s.termin_opis)}</small></div>`;
          })
          .join("")}
      </div>`;
    })
    .join("");
}

function rysujStaty() {
  const st = stan.smart.statystyki;
  const max = Math.max(1, ...st.obciazenie_14_dni.map((x) => x[1]));
  $("#staty").innerHTML = `
    <div class="kart"><h4>Podsumowanie</h4><div class="liczby">
      <div class="liczba"><b>${st.wykonane}/${st.wszystkie}</b><span>zrobione</span></div>
      <div class="liczba"><b>${st.procent}%</b><span>postęp</span></div>
      <div class="liczba"><b style="color:var(--zalegle)">${st.po_terminie}</b><span>po terminie</span></div>
      <div class="liczba"><b style="color:var(--akcent)">${st.na_dzis}</b><span>na dziś</span></div>
    </div></div>
    <div class="kart"><h4>Postęp wg tagów</h4>
      ${st.tagi
        .map((t) => {
          const p = Math.round((t.wykonane * 100) / t.wszystkie);
          return `<div class="wiersz"><span>#${esc(t.tag)}</span><div class="pasek"><span style="width:${p}%"></span></div><small>${t.wykonane}/${t.wszystkie}</small></div>`;
        })
        .join("")}
    </div>
    <div class="kart" style="grid-column: 1 / -1"><h4>Terminy w najbliższych 14 dniach</h4>
      <div class="slupki">${st.obciazenie_14_dni
        .map(([d, n]) => `<div class="slupek ${n >= 2 ? "duzo" : ""}"><em>${n || ""}</em><i style="height:${(n / max) * 100}%"></i><small>${esc(d)}</small></div>`)
        .join("")}</div>
    </div>`;
}

function podswietl(ids) {
  ids.forEach((id) => {
    const el = document.querySelector(`.zad[data-id="${id}"]`);
    if (el) {
      el.classList.remove("swieci");
      void el.offsetWidth;
      el.classList.add("swieci");
      if (ids[0] === id) el.scrollIntoView({ behavior: "smooth", block: "nearest" });
    }
  });
}

// ───────── toast z cofaniem ─────────
let toastTimer;
function toast(tekst, cofnij) {
  const t = $("#toast");
  t.innerHTML = `<span>${esc(tekst)}</span>${cofnij ? '<button id="toast-cofnij">Cofnij</button>' : ""}`;
  t.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (t.hidden = true), 5000);
  if (cofnij) $("#toast-cofnij").onclick = async () => { t.hidden = true; await cofnij(); };
}

// ───────── akcje na zadaniach ─────────
async function przelacz(id) {
  const z = await api(`/api/zadania/${id}/przelacz`, { method: "PATCH" });
  await odswiez();
  if (z.wykonane) toast(`✅ „${z.tytul}” zrobione`, () => przelacz(id).then(() => {}));
  podswietl([id]);
}

async function usun(id) {
  const z = stan.zadania.find((x) => x.id === id);
  await api(`/api/zadania/${id}`, { method: "DELETE" });
  await odswiez();
  toast(`🗑 Usunięto „${z.tytul}”`, async () => {
    const { id: _, ...reszta } = z;
    const nowe = await api("/api/zadania", { method: "POST", body: JSON.stringify(reszta) });
    await odswiez();
    podswietl([nowe.id]);
  });
}

function otworzOkno(z) {
  stan.edytowane = z ? z.id : null;
  const f = $("#formularz");
  f.reset();
  $("#okno-tytul").textContent = z ? "Edytuj zadanie" : "Nowe zadanie";
  $("#btn-usun-okno").hidden = !z;
  if (z) {
    f.tytul.value = z.tytul;
    f.opis.value = z.opis;
    f.termin.value = z.termin || "";
    f.querySelector(`[name=priorytet][value=${z.priorytet}]`).checked = true;
    f.tagi.value = z.tagi.join(", ");
    f.wykonane.checked = z.wykonane;
  }
  $("#okno").showModal();
  f.tytul.focus();
}

async function zapiszOkno(e) {
  e.preventDefault();
  const f = $("#formularz");
  const dane = {
    tytul: f.tytul.value.trim(),
    opis: f.opis.value.trim(),
    termin: f.termin.value || null,
    priorytet: f.querySelector("[name=priorytet]:checked").value,
    wykonane: f.wykonane.checked,
    tagi: f.tagi.value.split(",").map((t) => t.trim()).filter(Boolean),
  };
  if (!dane.tytul) return;
  try {
    const z = stan.edytowane
      ? await api(`/api/zadania/${stan.edytowane}`, { method: "PUT", body: JSON.stringify(dane) })
      : await api("/api/zadania", { method: "POST", body: JSON.stringify(dane) });
    $("#okno").close();
    await odswiez();
    podswietl([z.id]);
    toast(stan.edytowane ? "Zapisano zmiany" : `📌 Dodano „${z.tytul}”`);
  } catch (err) {
    toast("Błąd: " + err.message);
  }
}

// ───────── czat z Ogarem ─────────
function formatuj(tekst) {
  return tekst
    .split("\n")
    .map((l) => {
      const h = esc(l).replace(/\*\*(.+?)\*\*/g, "<b>$1</b>");
      return `<div class="${l.startsWith("•") ? "li" : ""}">${h || "&nbsp;"}</div>`;
    })
    .join("");
}

function dodajWiadomosc(kto, html) {
  const el = document.createElement("div");
  el.className = "msg " + kto;
  el.innerHTML = html;
  $("#wiadomosci").appendChild(el);
  $("#wiadomosci").scrollTop = 1e9;
  return el;
}

function ustawSugestie(lista) {
  $("#sugestie").innerHTML = (lista || []).map((s) => `<button>${esc(s)}</button>`).join("");
}

const czekaj = (ms) => new Promise((r) => setTimeout(r, ms));

async function wyslijDoOgara(tekst) {
  tekst = tekst.trim();
  if (!tekst) return;
  dodajWiadomosc("ja", esc(tekst));
  ustawSugestie([]);
  const pisze = dodajWiadomosc("ogar", '<span class="pisze"><i></i><i></i><i></i></span>');
  pisze.classList.add("pisze-box");
  let odp;
  try {
    [odp] = await Promise.all([
      api("/api/asystent", { method: "POST", body: JSON.stringify({ wiadomosc: tekst }) }),
      czekaj(550),
    ]);
  } catch (e) {
    pisze.remove();
    dodajWiadomosc("ogar", "Ups, coś nie tak z serwerem: " + esc(e.message));
    return;
  }
  pisze.remove();
  if (odp.mysli?.length) {
    const m = document.createElement("details");
    m.className = "mysli";
    m.innerHTML = `<summary>Ogar pomyślał (${odp.mysli.length} ${odp.mysli.length >= 5 ? "kroków" : "kroki"})</summary><ol>${odp.mysli.map((x) => `<li>${esc(x)}</li>`).join("")}</ol>`;
    $("#wiadomosci").appendChild(m);
  }
  // efekt „pisania” jak w prawdziwym czacie AI
  const el = dodajWiadomosc("ogar", "");
  const pelny = odp.tekst;
  const krok = Math.max(2, Math.round(pelny.length / 60));
  for (let i = krok; i < pelny.length + krok; i += krok) {
    el.innerHTML = formatuj(pelny.slice(0, i));
    $("#wiadomosci").scrollTop = 1e9;
    await czekaj(12);
  }
  ustawSugestie(odp.sugestie);
  if (odp.zmieniono) await odswiez();
  if (odp.podswietl?.length) {
    if (["tydzien", "statystyki"].includes(stan.widok)) { stan.widok = "teraz"; rysuj(); }
    podswietl(odp.podswietl);
  }
}

// ───────── zdarzenia ─────────
function podepnij() {
  $("#widoki").addEventListener("click", (e) => {
    const b = e.target.closest("button");
    if (!b) return;
    stan.widok = b.dataset.widok;
    rysuj();
  });
  $("#tagi-filtr").addEventListener("click", (e) => {
    const t = e.target.closest("[data-tag]");
    if (!t) return;
    stan.tag = stan.tag === t.dataset.tag ? null : t.dataset.tag;
    rysuj();
  });
  $("#szukaj").addEventListener("input", (e) => { stan.szukaj = e.target.value; rysujListe(); });
  $("#sortuj").addEventListener("change", (e) => { stan.sortuj = e.target.value; rysujListe(); });

  const klikZadania = (e) => {
    const zrob = e.target.closest("[data-zrob]");
    if (zrob) return przelacz(+zrob.dataset.zrob);
    const tag = e.target.closest(".tag[data-tag]");
    if (tag) { stan.tag = tag.dataset.tag; return rysuj(); }
    const akcja = e.target.closest("[data-akcja]");
    if (!akcja) return;
    const id = +akcja.closest("[data-id]").dataset.id;
    const z = stan.zadania.find((x) => x.id === id);
    if (akcja.dataset.akcja === "przelacz") przelacz(id);
    if (akcja.dataset.akcja === "edytuj") otworzOkno(z);
    if (akcja.dataset.akcja === "usun") usun(id);
  };
  $("#lista").addEventListener("click", klikZadania);
  $("#hero").addEventListener("click", klikZadania);
  $("#tydzien").addEventListener("click", klikZadania);

  $("#btn-nowe").onclick = () => otworzOkno(null);
  $("#btn-anuluj").onclick = () => $("#okno").close();
  $("#formularz").addEventListener("submit", zapiszOkno);
  $("#btn-usun-okno").onclick = () => { $("#okno").close(); usun(stan.edytowane); };
  document.querySelectorAll(".szybkie-daty button").forEach((b) => {
    b.onclick = () => { $("#formularz").termin.value = b.dataset.za === "" ? "" : dodajDni(+b.dataset.za); };
  });

  // szybkie dodawanie zdaniem: parsuje Ogar (Rust)
  $("#szybkie").addEventListener("submit", async (e) => {
    e.preventDefault();
    const pole = $("#szybkie-pole");
    const t = pole.value.trim();
    if (!t) return;
    pole.value = "";
    otworzCzat();
    await wyslijDoOgara(/^(dodaj|dopisz|zapisz|przypomnij)\b/i.test(t) ? t : "dodaj " + t);
  });

  $("#czat-form").addEventListener("submit", (e) => {
    e.preventDefault();
    const i = $("#czat-input");
    const t = i.value;
    i.value = "";
    wyslijDoOgara(t);
  });
  $("#sugestie").addEventListener("click", (e) => {
    const b = e.target.closest("button");
    if (b) wyslijDoOgara(b.textContent);
  });
  $("#btn-czat").onclick = () => $("#czat").classList.toggle("otwarty");
  $("#czat-zamknij").onclick = () => $("#czat").classList.remove("otwarty");

  $("#plik-import").addEventListener("change", async (e) => {
    const f = e.target.files[0];
    if (!f) return;
    try {
      const r = await api("/api/import", { method: "POST", body: await f.text(), raw: true });
      await odswiez();
      toast(`⬆ Wczytano ${r.wczytano} zadań z „${f.name}”`);
    } catch (err) {
      toast("Nie udało się wczytać: " + err.message);
    }
    e.target.value = "";
  });
  $("#btn-przyklady").onclick = async () => {
    if (!confirm("Zastąpić obecną listę 20 przykładowymi zadaniami?")) return;
    await api("/api/przyklady", { method: "POST" });
    await odswiez();
    toast("Przywrócono przykładowe dane");
  };

  document.addEventListener("keydown", (e) => {
    if (e.target.matches("input, textarea, select")) return;
    if (e.key === "n") { e.preventDefault(); otworzOkno(null); }
    if (e.key === "/") { e.preventDefault(); $("#czat-input").focus(); }
  });
}

function otworzCzat() {
  if (window.innerWidth <= 1180) $("#czat").classList.add("otwarty");
}

async function start() {
  podepnij();
  await odswiez();
  const st = stan.smart.statystyki;
  dodajWiadomosc(
    "ogar",
    formatuj(
      `Hej, tu **Ogar** 🐕 Węszę w twoich terminach.\nNa dziś: **${st.na_dzis}**, po terminie: **${st.po_terminie}**.\nNapisz, co masz na głowie, a ja to ogarnę.`
    )
  );
  ustawSugestie(["Co teraz?", "Zaplanuj mi tydzień", "Dodaj kolokwium z fizyki w czwartek", "Nie dam rady 😩"]);
}

start();
