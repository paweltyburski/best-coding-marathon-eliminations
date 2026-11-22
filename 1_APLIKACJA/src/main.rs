//! Ogarniacz: serwer HTTP (axum) z API zadań, widokami „smart” i asystentem Ogar.
//! Front (HTML/CSS/JS) jest wkompilowany w binarkę, więc wystarczy `cargo run`.

mod asystent;
mod model;
mod smart;

use asystent::{Mozg, RegulowyOgar, StanRozmowy};
use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, patch, post, put},
    Json, Router,
};
use chrono::{Local, NaiveDate};
use model::{NoweZadanie, Plik, Zadanie};
use serde::Deserialize;
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

struct Stan {
    plik: Plik,
    rozmowa: StanRozmowy,
    sciezka: PathBuf,
}

#[derive(Clone)]
struct App {
    stan: Arc<Mutex<Stan>>,
    mozg: Arc<dyn Mozg>,
}

const PRZYKLADY: &str = include_str!("../przyklady/zadania.json");

/// „Dziś”. Do nagrań demo można podać stałą datę: `OGARNIACZ_DZIS=2026-11-22`.
fn dzis() -> NaiveDate {
    std::env::var("OGARNIACZ_DZIS")
        .ok()
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| Local::now().date_naive())
}

impl Stan {
    fn zapisz(&self) {
        if let Some(dir) = self.sciezka.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match serde_json::to_string_pretty(&self.plik) {
            Ok(s) => {
                if let Err(e) = std::fs::write(&self.sciezka, s) {
                    eprintln!("Nie udało się zapisać {}: {e}", self.sciezka.display());
                }
            }
            Err(e) => eprintln!("Błąd serializacji: {e}"),
        }
    }
}

type Wynik<T> = Result<T, (StatusCode, Json<serde_json::Value>)>;

fn blad(kod: StatusCode, msg: impl Into<String>) -> (StatusCode, Json<serde_json::Value>) {
    (kod, Json(json!({ "blad": msg.into() })))
}

// ───────────────────────────── handlery ─────────────────────────────

async fn lista(State(app): State<App>) -> Json<Plik> {
    Json(app.stan.lock().await.plik.clone())
}

async fn dodaj(State(app): State<App>, Json(n): Json<NoweZadanie>) -> Wynik<Json<Zadanie>> {
    if n.tytul.trim().is_empty() {
        return Err(blad(StatusCode::BAD_REQUEST, "Tytuł nie może być pusty"));
    }
    let mut s = app.stan.lock().await;
    let z = s.plik.dodaj(n);
    s.zapisz();
    Ok(Json(z))
}

async fn edytuj(State(app): State<App>, Path(id): Path<u64>, Json(n): Json<NoweZadanie>) -> Wynik<Json<Zadanie>> {
    if n.tytul.trim().is_empty() {
        return Err(blad(StatusCode::BAD_REQUEST, "Tytuł nie może być pusty"));
    }
    let mut s = app.stan.lock().await;
    let z = s.plik.znajdz_mut(id).ok_or_else(|| blad(StatusCode::NOT_FOUND, "Nie ma takiego zadania"))?;
    z.tytul = n.tytul.trim().to_string();
    z.opis = n.opis.trim().to_string();
    z.termin = n.termin;
    z.priorytet = n.priorytet;
    z.wykonane = n.wykonane;
    z.tagi = model::oczysc_tagi(n.tagi);
    let z = z.clone();
    s.zapisz();
    Ok(Json(z))
}

async fn przelacz(State(app): State<App>, Path(id): Path<u64>) -> Wynik<Json<Zadanie>> {
    let mut s = app.stan.lock().await;
    let z = s.plik.znajdz_mut(id).ok_or_else(|| blad(StatusCode::NOT_FOUND, "Nie ma takiego zadania"))?;
    z.wykonane = !z.wykonane;
    let z = z.clone();
    s.zapisz();
    Ok(Json(z))
}

async fn usun(State(app): State<App>, Path(id): Path<u64>) -> Wynik<StatusCode> {
    let mut s = app.stan.lock().await;
    let przed = s.plik.zadania.len();
    s.plik.zadania.retain(|z| z.id != id);
    if s.plik.zadania.len() == przed {
        return Err(blad(StatusCode::NOT_FOUND, "Nie ma takiego zadania"));
    }
    s.zapisz();
    Ok(StatusCode::NO_CONTENT)
}

/// Import: treść pliku JSON w tym samym formacie. Zastępuje bieżącą listę.
async fn importuj(State(app): State<App>, body: String) -> Wynik<Json<serde_json::Value>> {
    let plik: Plik = serde_json::from_str(&body)
        .map_err(|e| blad(StatusCode::BAD_REQUEST, format!("To nie jest poprawny plik zadań: {e}")))?;
    let mut s = app.stan.lock().await;
    let n = plik.zadania.len();
    s.plik = plik;
    s.rozmowa = StanRozmowy::default();
    s.zapisz();
    Ok(Json(json!({ "wczytano": n })))
}

async fn eksportuj(State(app): State<App>) -> Response {
    let s = app.stan.lock().await;
    let body = serde_json::to_string_pretty(&s.plik).unwrap_or_default();
    (
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"zadania.json\""),
        ],
        body,
    )
        .into_response()
}

async fn przywroc_przyklady(State(app): State<App>) -> Json<serde_json::Value> {
    let plik: Plik = serde_json::from_str(PRZYKLADY).expect("przyklady/zadania.json jest poprawny");
    let mut s = app.stan.lock().await;
    let n = plik.zadania.len();
    s.plik = plik;
    s.rozmowa = StanRozmowy::default();
    s.zapisz();
    Json(json!({ "wczytano": n }))
}

async fn widok_smart(State(app): State<App>) -> Json<serde_json::Value> {
    let s = app.stan.lock().await;
    let d = dzis();
    let z = &s.plik.zadania;
    let ranking = smart::ranking(z, d);
    Json(json!({
        "dzis": d,
        "ranking": ranking,
        "plan": smart::plan_tygodnia(z, d),
        "statystyki": smart::statystyki(z, d),
        "ostrzezenia": smart::ostrzezenia(z, d),
    }))
}

#[derive(Deserialize)]
struct Wiadomosc {
    wiadomosc: String,
}

async fn asystent(State(app): State<App>, Json(w): Json<Wiadomosc>) -> Json<asystent::Odpowiedz> {
    let mut s = app.stan.lock().await;
    let Stan { plik, rozmowa, .. } = &mut *s;
    let odp = app.mozg.odpowiedz(&w.wiadomosc, plik, dzis(), rozmowa);
    if odp.zmieniono {
        s.zapisz();
    }
    Json(odp)
}

// ───────────────────────────── statyczne pliki ─────────────────────────────

async fn index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}
async fn css() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], include_str!("../static/app.css"))
}
async fn js() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "application/javascript; charset=utf-8")], include_str!("../static/app.js"))
}
async fn ikona() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "image/svg+xml")], include_str!("../static/ogar.svg"))
}

#[tokio::main]
async fn main() {
    let sciezka = PathBuf::from(std::env::var("OGARNIACZ_PLIK").unwrap_or_else(|_| "dane/zadania.json".into()));
    let plik: Plik = match std::fs::read_to_string(&sciezka) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            eprintln!("Plik {} jest uszkodzony ({e}), wczytuję przykłady.", sciezka.display());
            serde_json::from_str(PRZYKLADY).unwrap()
        }),
        Err(_) => serde_json::from_str(PRZYKLADY).unwrap(),
    };
    let stan = Stan { plik, rozmowa: StanRozmowy::default(), sciezka };
    stan.zapisz();

    let app = App { stan: Arc::new(Mutex::new(stan)), mozg: Arc::new(RegulowyOgar) };

    let router = Router::new()
        .route("/", get(index))
        .route("/app.css", get(css))
        .route("/app.js", get(js))
        .route("/ogar.svg", get(ikona))
        .route("/api/zadania", get(lista).post(dodaj))
        .route("/api/zadania/{id}", put(edytuj).delete(usun))
        .route("/api/zadania/{id}/przelacz", patch(przelacz))
        .route("/api/import", post(importuj))
        .route("/api/eksport", get(eksportuj))
        .route("/api/przyklady", post(przywroc_przyklady))
        .route("/api/smart", get(widok_smart))
        .route("/api/asystent", post(asystent))
        .with_state(app);

    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3000);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await.expect("port zajęty? ustaw PORT=…");
    println!("🐕 Ogarniacz działa: http://localhost:{port}  (dziś = {})", dzis());
    axum::serve(listener, router).await.unwrap();
}
