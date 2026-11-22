//! Ogar: atrapa asystenta AI.
//!
//! To NIE jest model językowy. Ogar rozumie polskie polecenia regułami (intencje,
//! daty mówione typu „w piątek”, „za 3 dni”, priorytety, tagi), ale ma ten sam
//! interfejs co prawdziwy asystent: trait [`Mozg`]. Podmiana na LLM = nowa
//! implementacja traitu, reszta aplikacji (API, czat w przeglądarce) zostaje.

use crate::model::{NoweZadanie, Plik, Priorytet, Zadanie};
use crate::smart::{self, capitalize, nazwa_dnia, odmiana_zadanie};
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use serde::Serialize;

#[derive(Serialize, Default, Debug)]
pub struct Odpowiedz {
    /// Tekst z prostym formatowaniem: `**pogrubienie**`, linie od `• ` to lista.
    pub tekst: String,
    /// „Tok myślenia” pokazywany w czacie (co Ogar rozpoznał).
    pub mysli: Vec<String>,
    /// Podpowiedzi następnego polecenia (przyciski pod odpowiedzią).
    pub sugestie: Vec<String>,
    /// Czy zmieniły się dane (front odświeża listę).
    pub zmieniono: bool,
    /// Zadania do podświetlenia na liście.
    pub podswietl: Vec<u64>,
}

/// Co Ogar zaproponował i czeka na „tak”.
#[derive(Debug, Clone)]
pub enum Oczekujace {
    Kroki { rodzic: String, kroki: Vec<NoweZadanie> },
    Usun { id: u64, tytul: String },
}

#[derive(Default)]
pub struct StanRozmowy {
    pub oczekuje: Option<Oczekujace>,
}

/// Interfejs „mózgu” asystenta. Dziś: reguły. Jutro: LLM z narzędziami.
pub trait Mozg: Send + Sync {
    fn odpowiedz(&self, wiadomosc: &str, plik: &mut Plik, dzis: NaiveDate, stan: &mut StanRozmowy) -> Odpowiedz;
}

pub struct RegulowyOgar;

// ───────────────────────────── normalizacja ─────────────────────────────

pub fn norm(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'ą' => 'a',
            'ć' => 'c',
            'ę' => 'e',
            'ł' => 'l',
            'ń' => 'n',
            'ó' => 'o',
            'ś' => 's',
            'ź' | 'ż' => 'z',
            _ => c,
        })
        .collect()
}

fn tokeny(s: &str) -> Vec<String> {
    s.split(|c: char| c.is_whitespace() || c == ',' || c == '!' || c == '?' || c == ';')
        .map(|t| t.trim_matches(|c: char| c == '"' || c == '„' || c == '”' || c == '\'' || c == ':' || c == '(' || c == ')'))
        .map(|t| t.trim_end_matches('.').to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

fn zawiera(n: &str, frazy: &[&str]) -> bool {
    frazy.iter().any(|f| n.contains(f))
}

// ───────────────────────────── daty ─────────────────────────────

fn dzien_tygodnia(t: &str) -> Option<Weekday> {
    let pary: [(&[&str], Weekday); 7] = [
        (&["poniedzialek", "poniedzialku", "pon"], Weekday::Mon),
        (&["wtorek", "wtorku", "wt"], Weekday::Tue),
        (&["sroda", "srode", "srody", "sr"], Weekday::Wed),
        (&["czwartek", "czwartku", "czw"], Weekday::Thu),
        (&["piatek", "piatku", "pt"], Weekday::Fri),
        (&["sobota", "sobote", "soboty", "sob"], Weekday::Sat),
        (&["niedziela", "niedziele", "niedzieli", "nd"], Weekday::Sun),
    ];
    pary.iter().find(|(w, _)| w.contains(&t)).map(|(_, d)| *d)
}

fn liczba(t: &str) -> Option<i64> {
    if let Ok(n) = t.parse::<i64>() {
        return Some(n);
    }
    let slowa = [
        ("jeden", 1), ("jeden", 1), ("dwa", 2), ("dwoch", 2), ("trzy", 3), ("trzech", 3),
        ("cztery", 4), ("czterech", 4), ("piec", 5), ("szesc", 6), ("siedem", 7), ("osiem", 8),
        ("dziewiec", 9), ("dziesiec", 10),
    ];
    slowa.iter().find(|(s, _)| *s == t).map(|(_, n)| *n)
}

fn miesiac(t: &str) -> Option<u32> {
    let m = [
        "stycznia", "lutego", "marca", "kwietnia", "maja", "czerwca", "lipca", "sierpnia",
        "wrzesnia", "pazdziernika", "listopada", "grudnia",
    ];
    m.iter().position(|x| *x == t).map(|i| i as u32 + 1)
}

fn nastepny(dzis: NaiveDate, w: Weekday) -> NaiveDate {
    let mut d = dzis + Duration::days(1);
    while d.weekday() != w {
        d += Duration::days(1);
    }
    d
}

fn rok_dla(dzis: NaiveDate, dzien: u32, mies: u32, rok: Option<i32>) -> Option<NaiveDate> {
    if let Some(r) = rok {
        return NaiveDate::from_ymd_opt(r, mies, dzien);
    }
    let d = NaiveDate::from_ymd_opt(dzis.year(), mies, dzien)?;
    if d < dzis - Duration::days(30) {
        NaiveDate::from_ymd_opt(dzis.year() + 1, mies, dzien)
    } else {
        Some(d)
    }
}

fn data_z_tokenu(t: &str, dzis: NaiveDate) -> Option<NaiveDate> {
    if let Ok(d) = NaiveDate::parse_from_str(t, "%Y-%m-%d") {
        return Some(d);
    }
    let czesci: Vec<&str> = t.split(|c| c == '.' || c == '/').filter(|s| !s.is_empty()).collect();
    if (2..=3).contains(&czesci.len()) && czesci.iter().all(|c| c.chars().all(|x| x.is_ascii_digit())) {
        let dz: u32 = czesci[0].parse().ok()?;
        let m: u32 = czesci[1].parse().ok()?;
        let r: Option<i32> = czesci.get(2).and_then(|r| r.parse().ok()).map(|r: i32| if r < 100 { 2000 + r } else { r });
        return rok_dla(dzis, dz, m, r);
    }
    None
}

/// Szuka terminu w tokenach. Zwraca datę, jej opis i zbiór „zużytych” indeksów.
fn znajdz_termin(n: &[String], dzis: NaiveDate) -> Option<(NaiveDate, String, Vec<usize>)> {
    let przyimki = ["na", "do", "w", "we", "dnia", "termin", "terminem", "z", "deadline"];
    let mut wynik: Option<(NaiveDate, String, Vec<usize>)> = None;
    let mut i = 0;
    while i < n.len() {
        let t = n[i].as_str();
        let mut trafienie: Option<(NaiveDate, String, usize)> = None; // (data, opis, ile tokenów)
        match t {
            "dzis" | "dzisiaj" => trafienie = Some((dzis, "dziś".into(), 1)),
            "jutro" => trafienie = Some((dzis + Duration::days(1), "jutro".into(), 1)),
            "pojutrze" => trafienie = Some((dzis + Duration::days(2), "pojutrze".into(), 1)),
            "za" => {
                if let Some(nast) = n.get(i + 1) {
                    if nast == "tydzien" {
                        trafienie = Some((dzis + Duration::days(7), "za tydzień".into(), 2));
                    } else if let Some(k) = liczba(nast) {
                        match n.get(i + 2).map(|s| s.as_str()) {
                            Some("dni") | Some("dzien") => {
                                trafienie = Some((dzis + Duration::days(k), format!("za {k} dni"), 3))
                            }
                            Some("tygodnie") | Some("tygodni") => {
                                trafienie = Some((dzis + Duration::days(7 * k), format!("za {k} tyg."), 3))
                            }
                            _ => {}
                        }
                    }
                }
            }
            "konca" if n.get(i + 1).map(|s| s.as_str()) == Some("tygodnia") => {
                let mut d = dzis;
                while d.weekday() != Weekday::Sun {
                    d += Duration::days(1);
                }
                trafienie = Some((d, "do końca tygodnia".into(), 2));
            }
            _ => {
                if let Some(w) = dzien_tygodnia(t) {
                    // krótkie skróty (pt, sr) tylko po przyimku, żeby nie łapać przypadkowych słów
                    let po_przyimku = i > 0 && przyimki.contains(&n[i - 1].as_str());
                    if t.len() > 3 || po_przyimku {
                        let d = nastepny(dzis, w);
                        trafienie = Some((d, nazwa_dnia(w).into(), 1));
                    }
                } else if let Some(d) = data_z_tokenu(t, dzis) {
                    trafienie = Some((d, d.format("%d.%m.%Y").to_string(), 1));
                } else if let (Ok(dz), Some(m)) = (t.parse::<u32>(), n.get(i + 1).and_then(|s| miesiac(s))) {
                    if let Some(d) = rok_dla(dzis, dz, m, None) {
                        trafienie = Some((d, d.format("%d.%m.%Y").to_string(), 2));
                    }
                }
            }
        }
        if let Some((d, opis, ile)) = trafienie {
            let mut zuzyte: Vec<usize> = (i..i + ile).collect();
            // zjedz przyimek przed („na piątek”, „do jutra”)
            let mut j = i;
            while j > 0 && przyimki.contains(&n[j - 1].as_str()) {
                j -= 1;
                zuzyte.push(j);
            }
            wynik = Some((d, opis, zuzyte));
            i += ile;
            continue;
        }
        i += 1;
    }
    wynik
}

// ───────────────────────────── priorytety i tagi ─────────────────────────────

fn znajdz_priorytet(n: &[String]) -> Option<(Priorytet, Vec<usize>)> {
    for (i, t) in n.iter().enumerate() {
        let p = match t.as_str() {
            "pilne" | "pilnie" | "pilny" | "wazne" | "wazny" | "asap" | "wysoki" | "wysokim" | "krytyczne" => {
                Some(Priorytet::Wysoki)
            }
            "niski" | "niskim" | "kiedys" | "luzno" | "niepilne" => Some(Priorytet::Niski),
            "sredni" | "srednim" | "normalny" | "normalnym" => Some(Priorytet::Sredni),
            _ => None,
        };
        if let Some(p) = p {
            let mut zuzyte = vec![i];
            if n.get(i + 1).map_or(false, |s| s.starts_with("prioryt")) {
                zuzyte.push(i + 1);
            }
            if i > 0 && (n[i - 1] == "z" || n[i - 1] == "bardzo" || n[i - 1] == "i") {
                zuzyte.push(i - 1);
            }
            if i > 0 && n[i - 1].starts_with("prioryt") {
                zuzyte.push(i - 1);
            }
            return Some((p, zuzyte));
        }
    }
    None
}

const SLOWA_STUDIA: &[&str] = &[
    "kolok", "egzam", "projekt", "labor", "laborka", "sprawozd", "wyklad", "cwicz", "referat", "prezentac",
    "esej", "zalicz", "kartkow", "semestr", "inzynier", "prace domowa", "zadanie domowe", "sieci", "matm",
    "analiz", "algebr", "fizyk", "programow",
];
const SLOWA_DOM: &[&str] = &["zakup", "pranie", "sprzat", "czynsz", "rachun", "obiad", "gotow", "naczyn", "smieci"];
const SLOWA_ZDROWIE: &[&str] = &["silown", "bieg", "trening", "basen", "lekarz", "dentyst", "rower", "joga"];
const SLOWA_PRACA: &[&str] = &["praca", "pracy", "zmiana", "klient", "faktur", "staz", "rekrutac", "cv"];

fn auto_tagi(tytul_n: &str) -> Vec<String> {
    let mut t = vec![];
    if zawiera(tytul_n, SLOWA_STUDIA) {
        t.push("studia".to_string());
    }
    if zawiera(tytul_n, SLOWA_DOM) {
        t.push("dom".to_string());
    }
    if zawiera(tytul_n, SLOWA_ZDROWIE) {
        t.push("zdrowie".to_string());
    }
    if zawiera(tytul_n, SLOWA_PRACA) {
        t.push("praca".to_string());
    }
    t
}

fn auto_priorytet(tytul_n: &str, termin: Option<NaiveDate>, dzis: NaiveDate) -> (Priorytet, Option<String>) {
    if zawiera(tytul_n, &["kolok", "egzam", "zalicz", "obron"]) {
        return (Priorytet::Wysoki, Some("sprawdzian, więc wysoki".into()));
    }
    if let Some(t) = termin {
        if (t - dzis).num_days() <= 1 {
            return (Priorytet::Wysoki, Some("termin za mniej niż 2 dni, więc wysoki".into()));
        }
    }
    if zawiera(tytul_n, &["projekt", "sprawozd", "referat", "prezentac"]) {
        return (Priorytet::Sredni, Some("praca na ocenę, więc średni".into()));
    }
    (Priorytet::Sredni, None)
}

// ───────────────────────────── wyszukiwanie zadania ─────────────────────────────

const SLOWA_POLECEN: &[&str] = &[
    "ogar", "hej", "prosze", "mozesz", "dodaj", "dopisz", "zapisz", "przypomnij", "mi", "o", "musze", "mam",
    "nowe", "zadanie", "rozbij", "podziel", "na", "kroki", "etapy", "jak", "sie", "zabrac", "do", "przygotowac",
    "zrobione", "zrobilem", "zrobilam", "skonczylem", "skonczylam", "odhacz", "wykonane", "gotowe", "oznacz", "jako",
    "usun", "skasuj", "wywal", "przesun", "przeloz", "zmien", "termin", "zadania", "to", "ten", "te", "z", "w", "i",
    "juz", "mam", "za", "sobą",
];

fn podobienstwo(zapytanie: &[String], tytul: &str) -> f32 {
    let tt: Vec<String> = tokeny(&norm(tytul));
    let znaczace: Vec<&String> = zapytanie.iter().filter(|q| q.len() >= 3).collect();
    if znaczace.is_empty() {
        return 0.0;
    }
    let mut trafione = 0.0;
    for q in &znaczace {
        let rdzen: String = q.chars().take(4).collect();
        if tt.iter().any(|t| t == *q) {
            trafione += 1.0;
        } else if tt.iter().any(|t| t.starts_with(&rdzen)) {
            trafione += 0.7;
        }
    }
    trafione / znaczace.len() as f32
}

fn znajdz_zadanie<'a>(n: &[String], zadania: &'a [Zadanie], tylko_niewykonane: bool) -> Option<&'a Zadanie> {
    let zapytanie: Vec<String> = n.iter().filter(|t| !SLOWA_POLECEN.contains(&t.as_str())).cloned().collect();
    zadania
        .iter()
        .filter(|z| !tylko_niewykonane || !z.wykonane)
        .map(|z| (z, podobienstwo(&zapytanie, &z.tytul)))
        .filter(|(_, s)| *s >= 0.5)
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(z, _)| z)
}

// ───────────────────────────── rozbijanie na kroki ─────────────────────────────

fn szablon_krokow(tytul_n: &str) -> (&'static str, Vec<&'static str>) {
    if zawiera(tytul_n, &["kolok", "egzam", "kartkow", "zalicz", "test"]) {
        (
            "sprawdzian",
            vec![
                "Zbierz materiały i listę zagadnień",
                "Przerób notatki, po jednym dziale na raz",
                "Rozwiąż zadania z poprzednich lat",
                "Próbne kolokwium na czas",
                "Powtórka najtrudniejszych tematów",
            ],
        )
    } else if zawiera(tytul_n, &["projekt", "aplikac", "program", "hackathon", "strona"]) {
        (
            "projekt",
            vec![
                "Spisz wymagania i podziel pracę",
                "Szkielet projektu i repozytorium",
                "Główna funkcja działa",
                "Testy i poprawki",
                "README i prezentacja",
            ],
        )
    } else if zawiera(tytul_n, &["sprawozd", "raport", "referat", "esej", "prezentac", "praca"]) {
        (
            "tekst",
            vec!["Zbierz dane i źródła", "Plan i struktura", "Pierwsza wersja", "Korekta, wykresy, formatowanie"],
        )
    } else {
        (
            "ogólne",
            vec!["Ustal, co znaczy „zrobione”", "Pierwszy mały krok (15 minut)", "Główna część pracy", "Sprawdź i domknij"],
        )
    }
}

fn skroc(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n - 1).collect::<String>().trim_end().to_string() + "…"
    }
}

fn zaplanuj_kroki(rodzic: &Zadanie, dzis: NaiveDate) -> Vec<NoweZadanie> {
    let (_, kroki) = szablon_krokow(&norm(&rodzic.tytul));
    let n = kroki.len() as i64;
    let dni_do = rodzic.termin.map(|t| (t - dzis).num_days()).unwrap_or(n);
    kroki
        .iter()
        .enumerate()
        .map(|(k, krok)| {
            let termin = if dni_do <= 0 {
                Some(dzis)
            } else {
                // równo rozłożone, ostatni krok dzień przed terminem (albo w dniu terminu przy krótkim czasie)
                let okno = (dni_do - 1).max(0);
                let off = if n <= 1 { okno } else { (k as i64 * okno) / (n - 1) };
                Some(dzis + Duration::days(off))
            };
            NoweZadanie {
                tytul: format!("{} › {}", skroc(&rodzic.tytul, 28), krok),
                opis: format!("Krok {} z {} dla: {}", k + 1, n, rodzic.tytul),
                termin,
                priorytet: rodzic.priorytet,
                wykonane: false,
                tagi: rodzic.tagi.clone(),
            }
        })
        .collect()
}

// ───────────────────────────── formatowanie ─────────────────────────────

fn linia(z: &Zadanie, dzis: NaiveDate) -> String {
    format!("• **{}**, {} ({})", z.tytul, smart::opis_terminu(z, dzis), z.priorytet.nazwa())
}

fn tak(n: &str) -> bool {
    let t = n.trim();
    ["tak", "ok", "okej", "jasne", "dawaj", "pewnie", "potwierdzam", "dodaj", "zrob to", "tak dodaj", "tak, dodaj", "usun", "tak usun", "yes", "jasne, dodaj"]
        .iter()
        .any(|x| t == *x || t.starts_with(&format!("{x} ")) || t.starts_with(&format!("{x},")))
}

fn nie(n: &str) -> bool {
    let t = n.trim();
    ["nie", "anuluj", "stop", "zostaw", "nie trzeba", "nie dzieki"].iter().any(|x| t == *x || t.starts_with(&format!("{x} ")))
}

// ───────────────────────────── główna logika ─────────────────────────────

impl Mozg for RegulowyOgar {
    fn odpowiedz(&self, wiadomosc: &str, plik: &mut Plik, dzis: NaiveDate, stan: &mut StanRozmowy) -> Odpowiedz {
        let n = norm(wiadomosc.trim());
        let tok_orig = tokeny(wiadomosc);
        let tok: Vec<String> = tok_orig.iter().map(|t| norm(t)).collect();

        // 1. Czy czekamy na potwierdzenie?
        if let Some(ocz) = stan.oczekuje.clone() {
            if tak(&n) {
                stan.oczekuje = None;
                return match ocz {
                    Oczekujace::Kroki { rodzic, kroki } => {
                        let mut ids = vec![];
                        let mut linie = vec![];
                        for k in kroki {
                            let z = plik.dodaj(k);
                            linie.push(linia(&z, dzis));
                            ids.push(z.id);
                        }
                        Odpowiedz {
                            tekst: format!(
                                "Gotowe, dodałem {} {} dla „{}”:\n{}\nOdhaczaj po kolei, a duże zadanie samo się zmniejszy.",
                                ids.len(),
                                odmiana_zadanie(ids.len()),
                                rodzic,
                                linie.join("\n")
                            ),
                            mysli: vec!["Potwierdzenie: tak".into(), format!("Zapisuję {} kroków z terminami", ids.len())],
                            sugestie: vec!["Zaplanuj mi tydzień".into(), "Co teraz?".into()],
                            zmieniono: true,
                            podswietl: ids,
                        }
                    }
                    Oczekujace::Usun { id, tytul } => {
                        plik.zadania.retain(|z| z.id != id);
                        Odpowiedz {
                            tekst: format!("Usunięte: **{tytul}**."),
                            mysli: vec!["Potwierdzenie: tak".into(), format!("Usuwam zadanie #{id}")],
                            sugestie: vec!["Co teraz?".into()],
                            zmieniono: true,
                            ..Default::default()
                        }
                    }
                };
            }
            if nie(&n) {
                stan.oczekuje = None;
                return Odpowiedz {
                    tekst: "Spoko, zostawiam jak było.".into(),
                    mysli: vec!["Potwierdzenie: nie, anuluję".into()],
                    sugestie: vec!["Co teraz?".into(), "Pomoc".into()],
                    ..Default::default()
                };
            }
            stan.oczekuje = None; // inne polecenie = porzucamy propozycję
        }

        let pytanie = matches!(tok.first().map(|s| s.as_str()), Some("co" | "jak" | "ile" | "kiedy" | "czy" | "ktore" | "jakie" | "pokaz"));

        // 2. Pomoc / small talk
        if n.is_empty() || zawiera(&n, &["pomoc", "co umiesz", "jak dzialasz", "help", "co potrafisz"]) {
            return pomoc();
        }
        if zawiera(&n, &["stres", "nie dam rady", "panik", "ratunku", "nie ogarniam", "za duzo", "nie wyrabiam", "pomocy"]) {
            return tryb_ratunkowy(plik, dzis);
        }
        if tok.len() <= 3 && zawiera(&n, &["czesc", "hej", "siema", "elo", "dzien dobry", "witaj", "hejka"]) {
            let st = smart::statystyki(&plik.zadania, dzis);
            return Odpowiedz {
                tekst: format!(
                    "Hej! Tu Ogar 🐕 Masz **{}** na dziś i **{}** po terminie. Mam ci powiedzieć, od czego zacząć?",
                    st.na_dzis, st.po_terminie
                ),
                mysli: vec!["Powitanie".into()],
                sugestie: vec!["Co teraz?".into(), "Zaplanuj mi tydzień".into(), "Pomoc".into()],
                ..Default::default()
            };
        }
        if zawiera(&n, &["dzieki", "dziekuje", "super", "git", "dobra robota"]) && tok.len() <= 4 {
            return Odpowiedz {
                tekst: "Do usług! Hau. Wołaj, jak coś się zacznie piętrzyć.".into(),
                mysli: vec!["Podziękowanie".into()],
                sugestie: vec!["Jak mi idzie?".into()],
                ..Default::default()
            };
        }

        // 3. Rozbij na kroki
        if zawiera(&n, &["rozbij", "podziel", "na kroki", "na etapy", "jak sie zabrac", "jak sie przygotowac"]) {
            return rozbij(&tok, plik, dzis, stan);
        }

        // 4. Odhacz
        if !pytanie && zawiera(&n, &["zrobione", "zrobilem", "zrobilam", "skonczylem", "skonczylam", "odhacz", "oznacz", "wykonane", "gotowe", "mam z glowy", "zaliczone"]) {
            return match znajdz_zadanie(&tok, &plik.zadania, true).map(|z| z.id) {
                Some(id) => {
                    let z = plik.znajdz_mut(id).unwrap();
                    z.wykonane = true;
                    let tytul = z.tytul.clone();
                    let st = smart::statystyki(&plik.zadania, dzis);
                    Odpowiedz {
                        tekst: format!("Brawo! ✅ **{tytul}** odhaczone. Jesteś na {}% całej listy.", st.procent),
                        mysli: vec!["Intencja: oznacz jako zrobione".into(), format!("Dopasowałem zadanie #{id}")],
                        sugestie: vec!["Co teraz?".into()],
                        zmieniono: true,
                        podswietl: vec![id],
                    }
                }
                None => nie_znalazlem("odhaczyć"),
            };
        }

        // 5. Usuń (z potwierdzeniem)
        if zawiera(&n, &["usun", "skasuj", "wywal"]) {
            return match znajdz_zadanie(&tok, &plik.zadania, false) {
                Some(z) => {
                    stan.oczekuje = Some(Oczekujace::Usun { id: z.id, tytul: z.tytul.clone() });
                    Odpowiedz {
                        tekst: format!("Na pewno usunąć **{}**? Tego nie da się cofnąć.", z.tytul),
                        mysli: vec!["Intencja: usuń".into(), format!("Dopasowałem zadanie #{}", z.id), "Pytam o potwierdzenie".into()],
                        sugestie: vec!["Tak, usuń".into(), "Nie".into()],
                        podswietl: vec![z.id],
                        ..Default::default()
                    }
                }
                None => nie_znalazlem("usunąć"),
            };
        }

        // 6. Przesuń termin
        if zawiera(&n, &["przesun", "przeloz", "zmien termin"]) {
            let termin = znajdz_termin(&tok, dzis);
            let bez_daty: Vec<String> = match &termin {
                Some((_, _, zuz)) => tok.iter().enumerate().filter(|(i, _)| !zuz.contains(i)).map(|(_, t)| t.clone()).collect(),
                None => tok.clone(),
            };
            return match (znajdz_zadanie(&bez_daty, &plik.zadania, true).map(|z| z.id), termin) {
                (Some(id), Some((d, opis, _))) => {
                    let z = plik.znajdz_mut(id).unwrap();
                    z.termin = Some(d);
                    let tytul = z.tytul.clone();
                    Odpowiedz {
                        tekst: format!("Przesunięte: **{tytul}** na {opis} ({}).", d.format("%d.%m")),
                        mysli: vec!["Intencja: zmiana terminu".into(), format!("Zadanie #{id}"), format!("Nowy termin: {opis} → {d}")],
                        sugestie: vec!["Zaplanuj mi tydzień".into()],
                        zmieniono: true,
                        podswietl: vec![id],
                    }
                }
                (None, _) => nie_znalazlem("przesunąć"),
                (Some(_), None) => Odpowiedz {
                    tekst: "Na kiedy? Napisz np. „przesuń pranie na sobotę” albo „…na 28.11”.".into(),
                    mysli: vec!["Intencja: zmiana terminu".into(), "Brak daty w poleceniu".into()],
                    ..Default::default()
                },
            };
        }

        // 7. Dodaj zadanie
        let pierwsze = tok.first().map(|s| s.as_str()).unwrap_or("");
        let start_dodaj = matches!(pierwsze, "dodaj" | "dopisz" | "zapisz" | "przypomnij" | "nowe" | "musze" | "mam" | "zanotuj")
            || (matches!(pierwsze, "ogar" | "hej") && tok.get(1).map_or(false, |t| matches!(t.as_str(), "dodaj" | "dopisz" | "zapisz" | "przypomnij")));
        let mam_pytanie = pierwsze == "mam" && znajdz_termin(&tok, dzis).is_none();
        if start_dodaj && !pytanie && !mam_pytanie {
            return dodaj(&tok_orig, &tok, plik, dzis);
        }

        // 8. Zapytania o stan
        if zawiera(&n, &["plan", "zaplanuj", "tydzien", "rozpisz", "grafik"]) {
            return plan(plik, dzis);
        }
        if zawiera(&n, &["co teraz", "od czego", "co najpierw", "co robic", "co mam robic", "co zrobic", "priorytet", "najwazniejsze"]) {
            return co_teraz(plik, dzis);
        }
        if zawiera(&n, &["po terminie", "zaleg", "spozni", "przegapi", "przeterminowane"]) {
            return po_terminie(plik, dzis);
        }
        if zawiera(&n, &["statyst", "jak mi idzie", "postep", "podsum", "ile zrobil", "wynik"]) {
            return statystyki(plik, dzis);
        }
        if zawiera(&n, &["jutro"]) {
            return na_dzien(plik, dzis, 1);
        }
        if zawiera(&n, &["dzis", "dzisiaj", "co mam"]) {
            return na_dzien(plik, dzis, 0);
        }

        // 9. Może pyta o konkretne zadanie?
        if let Some(z) = znajdz_zadanie(&tok, &plik.zadania, false) {
            let mut t = format!(
                "Znalazłem: **{}**\n• termin: {}\n• priorytet: {}\n• status: {}",
                z.tytul,
                smart::opis_terminu(z, dzis),
                z.priorytet.nazwa(),
                if z.wykonane { "zrobione ✅" } else { "do zrobienia" }
            );
            if !z.opis.is_empty() {
                t.push_str(&format!("\n• opis: {}", z.opis));
            }
            let krotki = skroc(&z.tytul, 30);
            return Odpowiedz {
                tekst: t,
                mysli: vec!["Nie rozpoznałem polecenia, szukam zadania o podobnej nazwie".into(), format!("Dopasowanie: #{}", z.id)],
                sugestie: vec![format!("Rozbij {krotki} na kroki"), format!("Zrobione {krotki}")],
                podswietl: vec![z.id],
                ..Default::default()
            };
        }

        Odpowiedz {
            tekst: "Hmm, tego jeszcze nie łapię 🐾 Jestem atrapą AI i rozumiem konkretne polecenia, np.:\n• „dodaj kolokwium z analizy w piątek”\n• „co teraz?”\n• „rozbij projekt na kroki”".into(),
            mysli: vec!["Nie rozpoznałem intencji".into()],
            sugestie: vec!["Pomoc".into(), "Co teraz?".into()],
            ..Default::default()
        }
    }
}

fn pomoc() -> Odpowiedz {
    Odpowiedz {
        tekst: "Jestem **Ogar** 🐕, twój pies tropiący terminy. Umiem:\n• dodać zadanie z luźnego zdania: „dodaj kolokwium z analizy w piątek pilne #studia”\n• powiedzieć, **co teraz** i dlaczego\n• **zaplanować tydzień**, tak żeby nic nie wybuchło naraz\n• **rozbić** duże zadanie na kroki z terminami\n• odhaczyć, przesunąć albo usunąć zadanie\n• pokazać zaległości i statystyki\n• a jak piszesz „nie dam rady”, włączam tryb ratunkowy".into(),
        mysli: vec!["Intencja: pomoc".into()],
        sugestie: vec![
            "Dodaj kolokwium z fizyki w czwartek".into(),
            "Co teraz?".into(),
            "Zaplanuj mi tydzień".into(),
            "Nie dam rady".into(),
        ],
        ..Default::default()
    }
}

fn nie_znalazlem(co: &str) -> Odpowiedz {
    Odpowiedz {
        tekst: format!("Nie wiem, które zadanie mam {co}. Podaj kawałek tytułu, np. „{co} pranie”."),
        mysli: vec!["Nie dopasowałem żadnego zadania".into()],
        ..Default::default()
    }
}

fn dodaj(tok_orig: &[String], tok: &[String], plik: &mut Plik, dzis: NaiveDate) -> Odpowiedz {
    let mut mysli = vec!["Intencja: nowe zadanie".to_string()];
    let mut zuzyte: Vec<usize> = vec![];

    let termin = znajdz_termin(tok, dzis);
    if let Some((d, opis, z)) = &termin {
        mysli.push(format!("Termin: „{opis}” → {}", d.format("%Y-%m-%d")));
        zuzyte.extend(z);
    }
    let prio_jawny = znajdz_priorytet(tok);
    if let Some((_, z)) = &prio_jawny {
        zuzyte.extend(z);
    }
    let mut tagi: Vec<String> = vec![];
    for (i, t) in tok_orig.iter().enumerate() {
        if let Some(tag) = t.strip_prefix('#') {
            if !tag.is_empty() {
                tagi.push(tag.to_lowercase());
                zuzyte.push(i);
            }
        }
    }

    // tytuł = reszta zdania bez słów polecenia na początku i bez spójników na końcu
    let mut slowa: Vec<(String, String)> = tok_orig
        .iter()
        .zip(tok.iter())
        .enumerate()
        .filter(|(i, _)| !zuzyte.contains(i))
        .map(|(_, (o, n))| (o.clone(), n.clone()))
        .collect();
    let wstep = ["ogar", "hej", "prosze", "dodaj", "dopisz", "zapisz", "zanotuj", "przypomnij", "mi", "o", "musze", "mam", "nowe", "zadanie", "zadanie:"];
    while slowa.first().map_or(false, |(_, n)| wstep.contains(&n.as_str())) {
        slowa.remove(0);
    }
    let koncowki = ["z", "na", "do", "w", "we", "i", "o", "-", "–", "priorytet", "priorytetem", "termin"];
    while slowa.last().map_or(false, |(_, n)| koncowki.contains(&n.as_str())) {
        slowa.pop();
    }
    let tytul = capitalize(&slowa.iter().map(|(o, _)| o.as_str()).collect::<Vec<_>>().join(" "));
    if tytul.is_empty() {
        return Odpowiedz {
            tekst: "Co mam dodać? Napisz np. „dodaj sprawozdanie z fizyki na 28.11”.".into(),
            mysli: vec!["Intencja: nowe zadanie".into(), "Brak tytułu".into()],
            ..Default::default()
        };
    }
    let tytul_n = norm(&tytul);
    let data = termin.as_ref().map(|t| t.0);
    let priorytet = match prio_jawny {
        Some((p, _)) => {
            mysli.push(format!("Priorytet: {} (powiedziałeś wprost)", p.nazwa()));
            p
        }
        None => {
            let (p, powod) = auto_priorytet(&tytul_n, data, dzis);
            mysli.push(match powod {
                Some(pw) => format!("Priorytet: {} ({pw})", p.nazwa()),
                None => format!("Priorytet: {} (domyślny)", p.nazwa()),
            });
            p
        }
    };
    let auto = auto_tagi(&tytul_n);
    for t in auto {
        if !tagi.contains(&t) {
            mysli.push(format!("Tag: #{t} (rozpoznany z treści)"));
            tagi.push(t);
        }
    }
    mysli.push(format!("Tytuł: „{tytul}”"));

    let z = plik.dodaj(NoweZadanie { tytul: tytul.clone(), opis: String::new(), termin: data, priorytet, wykonane: false, tagi });
    let kiedy = match &termin {
        Some((d, opis, _)) => format!("na **{opis}** ({})", d.format("%d.%m")),
        None => "bez terminu".to_string(),
    };
    let mut tekst = format!("Dodane! 📌 **{}**, {kiedy}, priorytet {}.", z.tytul, z.priorytet.nazwa());
    // ostrzeżenie o kumulacji tego dnia
    if let Some(d) = data {
        let tego_dnia = plik.zadania.iter().filter(|x| !x.wykonane && x.termin == Some(d)).count();
        if tego_dnia >= 3 {
            tekst.push_str(&format!("\nUwaga: tego dnia masz już {tego_dnia} terminy. Zaplanować to wcześniej?"));
        }
    }
    let krotki = skroc(&z.tytul, 30);
    Odpowiedz {
        tekst,
        mysli,
        sugestie: vec![format!("Rozbij {krotki} na kroki"), "Zaplanuj mi tydzień".into()],
        zmieniono: true,
        podswietl: vec![z.id],
    }
}

fn rozbij(tok: &[String], plik: &mut Plik, dzis: NaiveDate, stan: &mut StanRozmowy) -> Odpowiedz {
    let z = match znajdz_zadanie(tok, &plik.zadania, true) {
        Some(z) => z.clone(),
        None => {
            // brak nazwy: weź najpilniejsze
            match smart::ranking(&plik.zadania, dzis).first().and_then(|s| plik.zadania.iter().find(|z| z.id == s.id)) {
                Some(z) => z.clone(),
                None => return nie_znalazlem("rozbić"),
            }
        }
    };
    let (rodzaj, _) = szablon_krokow(&norm(&z.tytul));
    let kroki = zaplanuj_kroki(&z, dzis);
    let linie: Vec<String> = kroki
        .iter()
        .map(|k| {
            let krok = k.tytul.split(" › ").nth(1).unwrap_or(&k.tytul);
            format!("• {} ({})", krok, k.termin.map(|d| format!("{} {}", smart::krotka_nazwa_dnia(d.weekday()), d.format("%d.%m"))).unwrap_or_default())
        })
        .collect();
    let mysli = vec![
        "Intencja: rozbij na kroki".to_string(),
        format!("Zadanie #{}: „{}”", z.id, z.tytul),
        format!("Rodzaj: {rodzaj}, dobieram szablon kroków"),
        format!("Rozkładam {} kroków do {}", kroki.len(), z.termin.map(|t| t.format("%d.%m").to_string()).unwrap_or("końca tygodnia".into())),
    ];
    stan.oczekuje = Some(Oczekujace::Kroki { rodzic: z.tytul.clone(), kroki });
    Odpowiedz {
        tekst: format!(
            "**{}** wygląda groźnie, ale w kawałkach już nie:\n{}\nDodać te kroki jako osobne zadania?",
            z.tytul,
            linie.join("\n")
        ),
        mysli,
        sugestie: vec!["Tak, dodaj".into(), "Nie".into()],
        podswietl: vec![z.id],
        ..Default::default()
    }
}

fn co_teraz(plik: &Plik, dzis: NaiveDate) -> Odpowiedz {
    let r = smart::ranking(&plik.zadania, dzis);
    if r.is_empty() {
        return Odpowiedz {
            tekst: "Wszystko zrobione! 🎉 Idź na spacer, zasłużyłeś.".into(),
            mysli: vec!["Brak niewykonanych zadań".into()],
            ..Default::default()
        };
    }
    let top: Vec<_> = r.iter().take(3).collect();
    let mut t = format!("Zacznij od **{}**, bo {}.", top[0].tytul, top[0].dlaczego);
    if top.len() > 1 {
        t.push_str("\nPotem:");
        for s in &top[1..] {
            t.push_str(&format!("\n• **{}** ({})", s.tytul, s.dlaczego));
        }
    }
    t.push_str("\nUstaw 25 minut skupienia i jedziesz. Resztą się nie przejmuj, pilnuję.");
    let krotki = skroc(&top[0].tytul, 30);
    Odpowiedz {
        tekst: t,
        mysli: vec![
            "Intencja: co robić teraz".into(),
            format!("Liczę pilność {} zadań: priorytet × bliskość terminu", r.len()),
            format!("Najwyżej: „{}” ({} pkt)", top[0].tytul, top[0].pilnosc),
        ],
        sugestie: vec![format!("Rozbij {krotki} na kroki"), "Zaplanuj mi tydzień".into()],
        podswietl: top.iter().map(|s| s.id).collect(),
        ..Default::default()
    }
}

fn plan(plik: &Plik, dzis: NaiveDate) -> Odpowiedz {
    let p = smart::plan_tygodnia(&plik.zadania, dzis);
    let mut t = String::from("Twój tydzień, rozłożony tak, żeby nic nie wybuchło naraz:");
    for d in &p {
        if d.zadania.is_empty() {
            t.push_str(&format!("\n• **{}**: luz 😌", d.etykieta));
        } else {
            let lista: Vec<String> = d.zadania.iter().map(|s| s.tytul.clone()).collect();
            t.push_str(&format!("\n• **{}**: {}", d.etykieta, lista.join(", ")));
        }
    }
    let ostrz = smart::ostrzezenia(&plik.zadania, dzis);
    if let Some(o) = ostrz.iter().find(|o| o.contains("kumulacja")) {
        t.push_str(&format!("\n⚠️ {o}"));
    }
    Odpowiedz {
        tekst: t,
        mysli: vec![
            "Intencja: plan tygodnia".into(),
            "Każde zadanie stawiam przed terminem (wysoki priorytet 2 dni wcześniej)".into(),
            "Maks. 3 rzeczy dziennie, nadmiar przesuwam na wcześniejsze dni".into(),
        ],
        sugestie: vec!["Co teraz?".into(), "Pokaż zaległe".into()],
        ..Default::default()
    }
}

fn po_terminie(plik: &Plik, dzis: NaiveDate) -> Odpowiedz {
    let mut z: Vec<&Zadanie> = plik
        .zadania
        .iter()
        .filter(|z| !z.wykonane && smart::dni_do(z, dzis).map_or(false, |d| d < 0))
        .collect();
    z.sort_by_key(|z| z.termin);
    if z.is_empty() {
        return Odpowiedz {
            tekst: "Zero zaległości. Czysto! 🧹".into(),
            mysli: vec!["Intencja: zaległe".into()],
            sugestie: vec!["Co teraz?".into()],
            ..Default::default()
        };
    }
    let linie: Vec<String> = z.iter().map(|x| linia(x, dzis)).collect();
    Odpowiedz {
        tekst: format!(
            "Po terminie: {} {}.\n{}\nProponuję: zrób dziś jedno, resztę przesuń na realne daty (np. „przesuń {} na środę”).",
            z.len(),
            odmiana_zadanie(z.len()),
            linie.join("\n"),
            skroc(&norm(&z[0].tytul).split_whitespace().next().unwrap_or("").to_string(), 20)
        ),
        mysli: vec!["Intencja: zaległe".into(), format!("Znalazłem {} zaległych", z.len())],
        sugestie: vec!["Co teraz?".into(), "Zaplanuj mi tydzień".into()],
        podswietl: z.iter().map(|x| x.id).collect(),
        ..Default::default()
    }
}

fn na_dzien(plik: &Plik, dzis: NaiveDate, za: i64) -> Odpowiedz {
    let d = dzis + Duration::days(za);
    let kiedy = if za == 0 { "dziś" } else { "jutro" };
    let mut z: Vec<&Zadanie> = plik.zadania.iter().filter(|z| !z.wykonane && z.termin == Some(d)).collect();
    z.sort_by(|a, b| b.priorytet.cmp(&a.priorytet));
    let zalegle = plik.zadania.iter().filter(|z| !z.wykonane && smart::dni_do(z, dzis).map_or(false, |x| x < 0)).count();
    let mut t = if z.is_empty() {
        format!("Na {kiedy} nie masz żadnych terminów.")
    } else {
        format!("Na {kiedy}:\n{}", z.iter().map(|x| linia(x, dzis)).collect::<Vec<_>>().join("\n"))
    };
    if za == 0 && zalegle > 0 {
        t.push_str(&format!("\nDo tego {zalegle} {} po terminie.", odmiana_zadanie(zalegle)));
    }
    Odpowiedz {
        tekst: t,
        mysli: vec![format!("Intencja: zadania na {kiedy}"), format!("Filtr: termin = {d}")],
        sugestie: vec!["Co teraz?".into(), if za == 0 { "A jutro?".into() } else { "Zaplanuj mi tydzień".into() }],
        podswietl: z.iter().map(|x| x.id).collect(),
        ..Default::default()
    }
}

fn statystyki(plik: &Plik, dzis: NaiveDate) -> Odpowiedz {
    let s = smart::statystyki(&plik.zadania, dzis);
    let komentarz = match s.procent {
        0..=24 => "Rozgrzewka. Każde odhaczone zadanie się liczy!",
        25..=49 => "Dobre tempo, ćwiartka za tobą.",
        50..=74 => "Ponad połowa! Ogar merda ogonem 🐕",
        _ => "Prawie koniec, finisz!",
    };
    let tagi: Vec<String> = s.tagi.iter().take(4).map(|t| format!("• #{}: {}/{}", t.tag, t.wykonane, t.wszystkie)).collect();
    Odpowiedz {
        tekst: format!(
            "Zrobione **{}/{}** ({}%). Po terminie: {}, na dziś: {}.\n{}\n{}",
            s.wykonane, s.wszystkie, s.procent, s.po_terminie, s.na_dzis, tagi.join("\n"), komentarz
        ),
        mysli: vec!["Intencja: statystyki".into()],
        sugestie: vec!["Co teraz?".into()],
        ..Default::default()
    }
}

fn tryb_ratunkowy(plik: &Plik, dzis: NaiveDate) -> Odpowiedz {
    let r = smart::ranking(&plik.zadania, dzis);
    let top: Vec<_> = r.iter().take(3).collect();
    let reszta = r.len().saturating_sub(3);
    let lista: Vec<String> = top.iter().enumerate().map(|(i, s)| format!("{}. **{}** ({})", i + 1, s.tytul, s.dlaczego)).collect();
    Odpowiedz {
        tekst: format!(
            "Spokojnie, ogarniemy to razem 🐕 Tryb ratunkowy:\nWeź trzy wdechy. Teraz liczą się tylko te 3 rzeczy:\n{}\nPozostałe {} {} poczekają, nikt się nie obrazi. Zacznij od pierwszej, 25 minut, bez telefonu.",
            lista.join("\n"),
            reszta,
            odmiana_zadanie(reszta)
        ),
        mysli: vec![
            "Wykryłem stres w wiadomości".into(),
            "Tryb ratunkowy: zawężam listę do 3 najpilniejszych".into(),
        ],
        sugestie: vec![format!("Rozbij {} na kroki", top.first().map(|s| skroc(&s.tytul, 30)).unwrap_or_default()), "Zaplanuj mi tydzień".into()],
        podswietl: top.iter().map(|s| s.id).collect(),
        ..Default::default()
    }
}

#[cfg(test)]
mod testy {
    use super::*;

    fn dzis() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 22).unwrap() // niedziela
    }

    fn pusty() -> Plik {
        Plik { wersja: 1, zadania: vec![] }
    }

    #[test]
    fn dodaje_z_luznego_zdania() {
        let mut p = pusty();
        let mut s = StanRozmowy::default();
        let o = RegulowyOgar.odpowiedz("dodaj kolokwium z analizy w piątek #matma", &mut p, dzis(), &mut s);
        assert!(o.zmieniono);
        let z = &p.zadania[0];
        assert_eq!(z.tytul, "Kolokwium z analizy");
        assert_eq!(z.termin, NaiveDate::from_ymd_opt(2026, 11, 27));
        assert_eq!(z.priorytet, Priorytet::Wysoki);
        assert!(z.tagi.contains(&"matma".to_string()));
        assert!(z.tagi.contains(&"studia".to_string()));
    }

    #[test]
    fn rozumie_rozne_daty() {
        let d = dzis();
        let t = |s: &str| znajdz_termin(&tokeny(&norm(s)), d).map(|x| x.0);
        assert_eq!(t("na jutro"), NaiveDate::from_ymd_opt(2026, 11, 23));
        assert_eq!(t("za 3 dni"), NaiveDate::from_ymd_opt(2026, 11, 25));
        assert_eq!(t("do 05.12"), NaiveDate::from_ymd_opt(2026, 12, 5));
        assert_eq!(t("10 grudnia"), NaiveDate::from_ymd_opt(2026, 12, 10));
        assert_eq!(t("za tydzień"), NaiveDate::from_ymd_opt(2026, 11, 29));
        assert_eq!(t("w poniedziałek"), NaiveDate::from_ymd_opt(2026, 11, 23));
    }

    #[test]
    fn rozbija_i_dodaje_po_potwierdzeniu() {
        let mut p = pusty();
        let mut s = StanRozmowy::default();
        RegulowyOgar.odpowiedz("dodaj projekt z baz danych za 8 dni", &mut p, dzis(), &mut s);
        let o = RegulowyOgar.odpowiedz("rozbij projekt na kroki", &mut p, dzis(), &mut s);
        assert!(!o.zmieniono);
        assert!(s.oczekuje.is_some());
        let o = RegulowyOgar.odpowiedz("tak", &mut p, dzis(), &mut s);
        assert!(o.zmieniono);
        assert_eq!(p.zadania.len(), 6);
        assert!(p.zadania.iter().all(|z| z.termin.unwrap() <= NaiveDate::from_ymd_opt(2026, 11, 30).unwrap()));
    }

    #[test]
    fn odhacza_po_nazwie() {
        let mut p = pusty();
        let mut s = StanRozmowy::default();
        RegulowyOgar.odpowiedz("dodaj pranie na sobotę", &mut p, dzis(), &mut s);
        RegulowyOgar.odpowiedz("zrobione pranie", &mut p, dzis(), &mut s);
        assert!(p.zadania[0].wykonane);
    }

    #[test]
    fn pytanie_nie_dodaje_zadania() {
        let mut p = pusty();
        let mut s = StanRozmowy::default();
        let o = RegulowyOgar.odpowiedz("co mam na dziś?", &mut p, dzis(), &mut s);
        assert!(!o.zmieniono);
        assert!(p.zadania.is_empty());
    }
}
