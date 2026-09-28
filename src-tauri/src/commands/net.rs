//! Comandos de las herramientas de web y red.

use crate::error::AppResult;
use crate::tools::http::{self, HttpRequest, HttpResponse};
use crate::tools::net::{self, CronReport, RegexOptions, RegexReport};

#[tauri::command]
pub fn test_regex(
    pattern: String,
    haystack: String,
    options: RegexOptions,
) -> AppResult<RegexReport> {
    net::test_regex(&pattern, &haystack, &options)
}

#[tauri::command]
pub fn replace_regex(
    pattern: String,
    haystack: String,
    replacement: String,
    options: RegexOptions,
) -> AppResult<String> {
    net::replace_regex(&pattern, &haystack, &replacement, &options)
}

#[tauri::command]
pub fn parse_cron(expression: String, count: usize) -> AppResult<CronReport> {
    net::parse_cron(&expression, count)
}

#[tauri::command]
pub fn http_methods() -> &'static [&'static str] {
    http::METHODS
}

#[tauri::command]
pub async fn send_http(request: HttpRequest) -> AppResult<HttpResponse> {
    http::send(&request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_probador_de_regex_pasa_las_opciones() {
        let report = test_regex(
            r"hola".into(),
            "HOLA".into(),
            RegexOptions {
                case_insensitive: true,
                ..RegexOptions::default()
            },
        )
        .unwrap();

        assert_eq!(report.total, 1);
    }

    #[test]
    fn el_cron_limita_la_cantidad_pedida() {
        // Se pide una barbaridad y se recorta en vez de generar miles.
        let report = parse_cron("* * * * *".into(), 10_000).unwrap();
        assert!(report.next_runs.len() <= 50, "{}", report.next_runs.len());
    }

    #[test]
    fn la_lista_de_metodos_llega_entera() {
        assert!(http_methods().contains(&"GET"));
        assert!(http_methods().len() >= 5);
    }
}
