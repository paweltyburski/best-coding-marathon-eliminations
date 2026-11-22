//! Model danych: dokładnie ten sam format co `zadania.json` z treści zadania.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Priorytet {
    Niski,
    #[serde(alias = "średni")]
    Sredni,
    Wysoki,
}

impl Priorytet {
    pub fn waga(self) -> i64 {
        match self {
            Priorytet::Niski => 1,
            Priorytet::Sredni => 2,
            Priorytet::Wysoki => 3,
        }
    }
    pub fn nazwa(self) -> &'static str {
        match self {
            Priorytet::Niski => "niski",
            Priorytet::Sredni => "średni",
            Priorytet::Wysoki => "wysoki",
        }
    }
}

impl Default for Priorytet {
    fn default() -> Self {
        Priorytet::Sredni
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zadanie {
    pub id: u64,
    pub tytul: String,
    #[serde(default)]
    pub opis: String,
    #[serde(default)]
    pub termin: Option<NaiveDate>,
    #[serde(default)]
    pub priorytet: Priorytet,
    #[serde(default)]
    pub wykonane: bool,
    #[serde(default)]
    pub tagi: Vec<String>,
}

/// Cały plik: `{ "wersja": 1, "zadania": [...] }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plik {
    #[serde(default = "domyslna_wersja")]
    pub wersja: u32,
    #[serde(default)]
    pub zadania: Vec<Zadanie>,
}

fn domyslna_wersja() -> u32 {
    1
}

/// Dane z formularza (bez id).
#[derive(Debug, Clone, Deserialize)]
pub struct NoweZadanie {
    pub tytul: String,
    #[serde(default)]
    pub opis: String,
    #[serde(default)]
    pub termin: Option<NaiveDate>,
    #[serde(default)]
    pub priorytet: Priorytet,
    #[serde(default)]
    pub wykonane: bool,
    #[serde(default)]
    pub tagi: Vec<String>,
}

impl Plik {
    pub fn nastepne_id(&self) -> u64 {
        self.zadania.iter().map(|z| z.id).max().unwrap_or(0) + 1
    }

    pub fn dodaj(&mut self, n: NoweZadanie) -> Zadanie {
        let z = Zadanie {
            id: self.nastepne_id(),
            tytul: n.tytul.trim().to_string(),
            opis: n.opis.trim().to_string(),
            termin: n.termin,
            priorytet: n.priorytet,
            wykonane: n.wykonane,
            tagi: oczysc_tagi(n.tagi),
        };
        self.zadania.push(z.clone());
        z
    }

    pub fn znajdz_mut(&mut self, id: u64) -> Option<&mut Zadanie> {
        self.zadania.iter_mut().find(|z| z.id == id)
    }
}

pub fn oczysc_tagi(tagi: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in tagi {
        let t = t.trim().trim_start_matches('#').to_lowercase();
        if !t.is_empty() && !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

#[cfg(test)]
mod testy {
    use super::*;

    #[test]
    fn wczytuje_i_zapisuje_w_tym_samym_formacie() {
        let wejscie = r#"{
          "wersja": 1,
          "zadania": [
            { "id": 1, "tytul": "Kolokwium z sieci komputerowych", "opis": "Rozdziały 3–5",
              "termin": "2026-11-18", "priorytet": "wysoki", "wykonane": false, "tagi": ["studia"] },
            { "id": 2, "tytul": "Pranie", "opis": "", "termin": null, "priorytet": "niski",
              "wykonane": true, "tagi": [] },
            { "id": 3, "tytul": "Bez opisu", "termin": null, "priorytet": "sredni", "wykonane": false, "tagi": [] }
          ]
        }"#;
        let plik: Plik = serde_json::from_str(wejscie).unwrap();
        assert_eq!(plik.zadania.len(), 3);
        assert_eq!(plik.zadania[0].priorytet, Priorytet::Wysoki);
        assert!(plik.zadania[1].termin.is_none());
        let wyjscie = serde_json::to_value(&plik).unwrap();
        assert_eq!(wyjscie["zadania"][0]["termin"], "2026-11-18");
        assert_eq!(wyjscie["zadania"][1]["termin"], serde_json::Value::Null);
        assert_eq!(wyjscie["zadania"][2]["priorytet"], "sredni");
        assert_eq!(wyjscie["zadania"][2]["opis"], "");
        // dokładnie te same klucze co w formacie wejściowym
        let klucze: Vec<_> = wyjscie["zadania"][0].as_object().unwrap().keys().cloned().collect();
        for k in ["id", "tytul", "opis", "termin", "priorytet", "wykonane", "tagi"] {
            assert!(klucze.contains(&k.to_string()));
        }
        assert_eq!(klucze.len(), 7);
    }
}
