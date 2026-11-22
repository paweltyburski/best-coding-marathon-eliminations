//! „Smart” część: pilność, widoki, plan tygodnia, statystyki i ostrzeżenia.

use crate::model::{Priorytet, Zadanie};
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use serde::Serialize;
use std::collections::BTreeMap;

/// Ile dni zostało do terminu (ujemne = po terminie).
pub fn dni_do(z: &Zadanie, dzis: NaiveDate) -> Option<i64> {
    z.termin.map(|t| (t - dzis).num_days())
}

pub fn opis_terminu(z: &Zadanie, dzis: NaiveDate) -> String {
    match dni_do(z, dzis) {
        None => "bez terminu".into(),
        Some(d) if d < -1 => format!("{} dni po terminie", -d),
        Some(-1) => "wczoraj minął termin".into(),
        Some(0) => "dziś".into(),
        Some(1) => "jutro".into(),
        Some(d) if d < 7 => nazwa_dnia(z.termin.unwrap().weekday()).to_string(),
        Some(d) => format!("za {} dni", d),
    }
}

pub fn nazwa_dnia(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "poniedziałek",
        Weekday::Tue => "wtorek",
        Weekday::Wed => "środa",
        Weekday::Thu => "czwartek",
        Weekday::Fri => "piątek",
        Weekday::Sat => "sobota",
        Weekday::Sun => "niedziela",
    }
}

pub fn krotka_nazwa_dnia(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "pon",
        Weekday::Tue => "wt",
        Weekday::Wed => "śr",
        Weekday::Thu => "czw",
        Weekday::Fri => "pt",
        Weekday::Sat => "sob",
        Weekday::Sun => "nd",
    }
}

/// Wynik pilności 0..~200: priorytet × bliskość terminu. Im wyżej, tym wcześniej to zrób.
pub fn pilnosc(z: &Zadanie, dzis: NaiveDate) -> i64 {
    if z.wykonane {
        return 0;
    }
    let w = z.priorytet.waga();
    let bliskosc = match dni_do(z, dzis) {
        None => 2,
        Some(d) if d < 0 => 40 + (-d).min(10) * 2, // zaległe rosną, ale z sufitem
        Some(d) => (14 - d.min(14)) * 3,           // 0..42
    };
    w * 12 + bliskosc * w / 2 + bliskosc
}

/// Krótkie, ludzkie uzasadnienie wyniku.
pub fn dlaczego(z: &Zadanie, dzis: NaiveDate) -> String {
    let p = z.priorytet.nazwa();
    match dni_do(z, dzis) {
        Some(d) if d < 0 => format!("{}, priorytet {p}", opis_terminu(z, dzis)),
        Some(0) => format!("termin dziś, priorytet {p}"),
        Some(1) => format!("termin jutro, priorytet {p}"),
        Some(d) if d < 7 => format!("termin za {d} dni ({}), priorytet {p}", nazwa_dnia(z.termin.unwrap().weekday())),
        Some(d) => format!("termin za {d} dni, priorytet {p}"),
        None => format!("bez terminu, priorytet {p}"),
    }
}

#[derive(Serialize, Clone)]
pub struct ZadanieSmart {
    pub id: u64,
    pub tytul: String,
    pub pilnosc: i64,
    pub termin_opis: String,
    pub dlaczego: String,
}

pub fn smart(z: &Zadanie, dzis: NaiveDate) -> ZadanieSmart {
    ZadanieSmart {
        id: z.id,
        tytul: z.tytul.clone(),
        pilnosc: pilnosc(z, dzis),
        termin_opis: opis_terminu(z, dzis),
        dlaczego: dlaczego(z, dzis),
    }
}

/// Niewykonane zadania posortowane od najpilniejszego.
pub fn ranking(zadania: &[Zadanie], dzis: NaiveDate) -> Vec<ZadanieSmart> {
    let mut v: Vec<ZadanieSmart> = zadania
        .iter()
        .filter(|z| !z.wykonane)
        .map(|z| smart(z, dzis))
        .collect();
    v.sort_by(|a, b| b.pilnosc.cmp(&a.pilnosc).then(a.id.cmp(&b.id)));
    v
}

#[derive(Serialize, Clone)]
pub struct DzienPlanu {
    pub data: NaiveDate,
    pub etykieta: String,
    pub zadania: Vec<ZadanieSmart>,
    pub terminy: usize,
    pub obciazenie: i64,
}

/// Plan na 7 dni: każde zadanie trafia na dzień PRZED terminem (wysoki priorytet
/// 2 dni wcześniej, średni 1), a dni są wyrównywane, żeby żaden nie miał więcej niż 3 rzeczy.
pub fn plan_tygodnia(zadania: &[Zadanie], dzis: NaiveDate) -> Vec<DzienPlanu> {
    const DNI: i64 = 7;
    const MAX_NA_DZIEN: usize = 3;
    let mut dni: Vec<DzienPlanu> = (0..DNI)
        .map(|i| {
            let data = dzis + Duration::days(i);
            let etykieta = match i {
                0 => "Dziś".to_string(),
                1 => "Jutro".to_string(),
                _ => format!("{} {}", krotka_nazwa_dnia(data.weekday()), data.format("%d.%m")),
            };
            DzienPlanu { data, etykieta, zadania: vec![], terminy: 0, obciazenie: 0 }
        })
        .collect();

    for z in zadania.iter().filter(|z| !z.wykonane) {
        if let Some(d) = dni_do(z, dzis) {
            if (0..DNI).contains(&d) {
                dni[d as usize].terminy += 1;
            }
        }
    }

    // kolejność: najpierw najpilniejsze, żeby dostały najlepsze miejsca
    let mut kolejka: Vec<&Zadanie> = zadania.iter().filter(|z| !z.wykonane).collect();
    kolejka.sort_by(|a, b| pilnosc(b, dzis).cmp(&pilnosc(a, dzis)));

    for z in kolejka {
        let docelowy = match dni_do(z, dzis) {
            Some(d) if d < 0 => 0,
            Some(d) => {
                let wyprzedzenie = match z.priorytet {
                    Priorytet::Wysoki => 2,
                    Priorytet::Sredni => 1,
                    Priorytet::Niski => 0,
                };
                (d - wyprzedzenie).max(0)
            }
            None => -1, // bez terminu: wstawimy na najlżejszy dzień
        };
        if docelowy >= DNI {
            continue; // termin za ponad tydzień: nie zaśmiecamy planu
        }
        let idx = if docelowy < 0 {
            // najlżejszy dzień, ale tylko niski/średni priorytet bez terminu zostawiamy na koniec tygodnia
            (0..DNI as usize).min_by_key(|&i| (dni[i].zadania.len(), dni[i].terminy)).unwrap()
        } else {
            // szukaj od docelowego wstecz (wcześniej = bezpieczniej), potem w przód
            let d = docelowy as usize;
            let mut wybor = None;
            for i in (0..=d).rev() {
                if dni[i].zadania.len() < MAX_NA_DZIEN {
                    wybor = Some(i);
                    break;
                }
            }
            // brak miejsca przed terminem: zaległe idą na najbliższy wolny dzień, reszta zostaje (nie przesuwamy za termin)
            let zalegle = dni_do(z, dzis).map_or(false, |x| x < 0);
            if wybor.is_none() && zalegle {
                wybor = (d + 1..DNI as usize).find(|&i| dni[i].zadania.len() < MAX_NA_DZIEN);
            }
            wybor.unwrap_or(d)
        };
        if dni[idx].zadania.len() >= MAX_NA_DZIEN + 1 && docelowy < 0 {
            continue;
        }
        let s = smart(z, dzis);
        dni[idx].obciazenie += z.priorytet.waga();
        dni[idx].zadania.push(s);
    }
    dni
}

#[derive(Serialize)]
pub struct TagStat {
    pub tag: String,
    pub wszystkie: usize,
    pub wykonane: usize,
}

#[derive(Serialize)]
pub struct Statystyki {
    pub wszystkie: usize,
    pub wykonane: usize,
    pub po_terminie: usize,
    pub na_dzis: usize,
    pub procent: u32,
    pub tagi: Vec<TagStat>,
    pub obciazenie_14_dni: Vec<(String, usize)>,
}

pub fn statystyki(zadania: &[Zadanie], dzis: NaiveDate) -> Statystyki {
    let wszystkie = zadania.len();
    let wykonane = zadania.iter().filter(|z| z.wykonane).count();
    let po_terminie = zadania
        .iter()
        .filter(|z| !z.wykonane && dni_do(z, dzis).map_or(false, |d| d < 0))
        .count();
    let na_dzis = zadania
        .iter()
        .filter(|z| !z.wykonane && dni_do(z, dzis) == Some(0))
        .count();
    let mut mapa: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for z in zadania {
        let tagi = if z.tagi.is_empty() { vec!["bez tagu".to_string()] } else { z.tagi.clone() };
        for t in tagi {
            let e = mapa.entry(t).or_default();
            e.0 += 1;
            if z.wykonane {
                e.1 += 1;
            }
        }
    }
    let mut tagi: Vec<TagStat> = mapa
        .into_iter()
        .map(|(tag, (w, k))| TagStat { tag, wszystkie: w, wykonane: k })
        .collect();
    tagi.sort_by(|a, b| b.wszystkie.cmp(&a.wszystkie));
    let obciazenie_14_dni = (0..14)
        .map(|i| {
            let d = dzis + Duration::days(i);
            let n = zadania.iter().filter(|z| !z.wykonane && z.termin == Some(d)).count();
            (format!("{} {}", krotka_nazwa_dnia(d.weekday()), d.format("%d")), n)
        })
        .collect();
    Statystyki {
        wszystkie,
        wykonane,
        po_terminie,
        na_dzis,
        procent: if wszystkie == 0 { 0 } else { (wykonane * 100 / wszystkie) as u32 },
        tagi,
        obciazenie_14_dni,
    }
}

/// Ostrzeżenia „zanim będzie za późno”: dni-kumulacje i zaległości.
pub fn ostrzezenia(zadania: &[Zadanie], dzis: NaiveDate) -> Vec<String> {
    let mut out = vec![];
    let zalegle: Vec<&Zadanie> = zadania
        .iter()
        .filter(|z| !z.wykonane && dni_do(z, dzis).map_or(false, |d| d < 0))
        .collect();
    if !zalegle.is_empty() {
        out.push(format!(
            "Masz {} {} po terminie. Zacznij od: „{}”.",
            zalegle.len(),
            odmiana_zadanie(zalegle.len()),
            zalegle.iter().max_by_key(|z| pilnosc(z, dzis)).unwrap().tytul
        ));
    }
    // kumulacje: dzień w ciągu 10 dni z ≥2 terminami, w tym wysokim (pokazujemy najwyżej 2)
    for i in 0..10 {
        if out.len() >= 3 {
            break;
        }
        let d = dzis + Duration::days(i);
        let tego_dnia: Vec<&Zadanie> = zadania
            .iter()
            .filter(|z| !z.wykonane && z.termin == Some(d))
            .collect();
        let wysokie = tego_dnia.iter().filter(|z| z.priorytet == Priorytet::Wysoki).count();
        if tego_dnia.len() >= 3 || (tego_dnia.len() >= 2 && wysokie >= 1) {
            let kiedy = match i {
                0 => "Dziś".to_string(),
                1 => "Jutro".to_string(),
                _ => format!("{} {}", capitalize(nazwa_dnia(d.weekday())), d.format("%d.%m")),
            };
            out.push(format!(
                "{kiedy} kumulacja: {} {} naraz. Zacznij „{}” wcześniej.",
                tego_dnia.len(),
                odmiana_termin(tego_dnia.len()),
                tego_dnia.iter().max_by_key(|z| z.priorytet).unwrap().tytul
            ));
        }
    }
    out
}

pub fn odmiana_zadanie(n: usize) -> &'static str {
    odmiana(n, "zadanie", "zadania", "zadań")
}
pub fn odmiana_termin(n: usize) -> &'static str {
    odmiana(n, "termin", "terminy", "terminów")
}

pub fn odmiana(n: usize, jeden: &'static str, kilka: &'static str, wiele: &'static str) -> &'static str {
    if n == 1 {
        return jeden;
    }
    let r10 = n % 10;
    let r100 = n % 100;
    if (2..=4).contains(&r10) && !(12..=14).contains(&r100) {
        kilka
    } else {
        wiele
    }
}

pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod testy {
    use super::*;

    fn z(id: u64, termin: Option<&str>, p: Priorytet) -> Zadanie {
        Zadanie {
            id,
            tytul: format!("Zadanie {id}"),
            opis: String::new(),
            termin: termin.map(|t| NaiveDate::parse_from_str(t, "%Y-%m-%d").unwrap()),
            priorytet: p,
            wykonane: false,
            tagi: vec![],
        }
    }

    #[test]
    fn zalegle_i_bliskie_sa_pilniejsze() {
        let dzis = NaiveDate::from_ymd_opt(2026, 11, 22).unwrap();
        let zalegle = z(1, Some("2026-11-20"), Priorytet::Sredni);
        let jutro = z(2, Some("2026-11-23"), Priorytet::Wysoki);
        let daleko = z(3, Some("2026-12-20"), Priorytet::Wysoki);
        let brak = z(4, None, Priorytet::Niski);
        assert!(pilnosc(&jutro, dzis) > pilnosc(&daleko, dzis));
        assert!(pilnosc(&zalegle, dzis) > pilnosc(&brak, dzis));
        assert!(pilnosc(&daleko, dzis) > pilnosc(&brak, dzis));
    }

    #[test]
    fn plan_nie_przekracza_limitu_przed_terminem() {
        let dzis = NaiveDate::from_ymd_opt(2026, 11, 22).unwrap();
        let zad: Vec<Zadanie> = (1..=6).map(|i| z(i, Some("2026-11-25"), Priorytet::Wysoki)).collect();
        let plan = plan_tygodnia(&zad, dzis);
        let suma: usize = plan.iter().map(|d| d.zadania.len()).sum();
        assert_eq!(suma, 6);
        assert!(plan.iter().all(|d| d.zadania.len() <= 3));
    }

    #[test]
    fn odmiana_po_polsku() {
        assert_eq!(odmiana_zadanie(1), "zadanie");
        assert_eq!(odmiana_zadanie(3), "zadania");
        assert_eq!(odmiana_zadanie(5), "zadań");
        assert_eq!(odmiana_zadanie(12), "zadań");
        assert_eq!(odmiana_zadanie(22), "zadania");
    }
}
