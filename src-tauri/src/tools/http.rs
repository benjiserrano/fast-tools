//! Cliente HTTP.
//!
//! Es la única herramienta de fast-tools que envía datos fuera de la máquina, y
//! siempre porque el usuario pulsa «Enviar». Por eso lleva límites propios:
//!
//! - Solo `http` y `https`. Un esquema como `file://` convertiría el cliente en
//!   un lector de archivos arbitrarios disfrazado de petición.
//! - Plazo máximo, para que una petición colgada no deje la pantalla bloqueada.
//! - Tope de respuesta, para que un servidor que envíe sin parar no llene la
//!   memoria.
//! - Sin redirecciones automáticas: se muestran y el usuario decide. Seguirlas
//!   en silencio manda las cabeceras —y las credenciales— a un destino que
//!   nunca escribió.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
const MAX_TIMEOUT_SECONDS: u64 = 300;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Seguir redirecciones en lugar de pararse en la primera.
    #[serde(default)]
    pub follow_redirects: bool,
}

fn default_timeout() -> u64 {
    30
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    /// Verdadero si el cuerpo no es texto y se ha resumido.
    pub body_is_binary: bool,
    pub bytes: usize,
    pub elapsed_ms: u128,
    /// Tipo de contenido tal como lo declara el servidor.
    pub content_type: Option<String>,
    /// A dónde redirige, cuando la respuesta es una redirección.
    pub redirect_to: Option<String>,
    pub truncated: bool,
}

/// Métodos que se ofrecen. `CONNECT` y `TRACE` quedan fuera: el primero es para
/// túneles de proxy y el segundo está desaconsejado por filtrar cabeceras.
pub const METHODS: &[&str] = &[
    "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS",
];

pub async fn send(request: &HttpRequest) -> AppResult<HttpResponse> {
    let url = request.url.trim();
    if url.is_empty() {
        return Err(AppError::msg("escribe una dirección"));
    }

    let parsed = url::Url::parse(url)
        .map_err(|e| AppError::msg(format!("La dirección no es válida: {e}")))?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(AppError::msg(format!(
            "solo se admiten direcciones http y https, y esta usa «{}»",
            parsed.scheme()
        )));
    }

    let method = request.method.trim().to_ascii_uppercase();
    if !METHODS.contains(&method.as_str()) {
        return Err(AppError::msg(format!("método no admitido: «{method}»")));
    }
    let method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|e| AppError::msg(format!("método no válido: {e}")))?;

    let timeout = Duration::from_secs(request.timeout_seconds.clamp(1, MAX_TIMEOUT_SECONDS));

    let client = reqwest::Client::builder()
        .user_agent(concat!("fast-tools/", env!("CARGO_PKG_VERSION")))
        .timeout(timeout)
        .redirect(if request.follow_redirects {
            reqwest::redirect::Policy::limited(10)
        } else {
            reqwest::redirect::Policy::none()
        })
        .build()
        .map_err(|e| AppError::msg(format!("no se pudo preparar la petición: {e}")))?;

    let mut builder = client.request(method, parsed);
    for (name, value) in &request.headers {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        builder = builder.header(name, value.trim());
    }
    if let Some(body) = &request.body {
        if !body.is_empty() {
            builder = builder.body(body.clone());
        }
    }

    let started = Instant::now();
    let response = builder.send().await.map_err(|e| {
        AppError::msg(if e.is_timeout() {
            format!("La petición ha superado el plazo de {} s.", timeout.as_secs())
        } else if e.is_connect() {
            format!("No se pudo conectar: {e}")
        } else {
            format!("Error al enviar la petición: {e}")
        })
    })?;

    let status = response.status();
    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.to_string(),
                value.to_str().unwrap_or("<valor no textual>").to_string(),
            )
        })
        .collect();

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let redirect_to = response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let raw = response
        .bytes()
        .await
        .map_err(|e| AppError::msg(format!("no se pudo leer la respuesta: {e}")))?;

    let bytes = raw.len();
    let truncated = bytes > MAX_RESPONSE_BYTES;
    let slice = &raw[..bytes.min(MAX_RESPONSE_BYTES)];

    // Una respuesta binaria pegada en un editor de texto sale ilegible y puede
    // arrastrar la interfaz; se resume en lugar de volcarla.
    let (body, body_is_binary) = match std::str::from_utf8(slice) {
        Ok(text) => (text.to_string(), false),
        Err(_) => (
            format!(
                "[{} bytes de datos binarios{}]",
                bytes,
                content_type
                    .as_deref()
                    .map(|kind| format!(", {kind}"))
                    .unwrap_or_default()
            ),
            true,
        ),
    };

    Ok(HttpResponse {
        status: status.as_u16(),
        status_text: status
            .canonical_reason()
            .unwrap_or("Sin descripción")
            .to_string(),
        headers,
        body,
        body_is_binary,
        bytes,
        elapsed_ms: started.elapsed().as_millis(),
        content_type,
        redirect_to,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(url: &str) -> HttpRequest {
        HttpRequest {
            method: "GET".into(),
            url: url.into(),
            headers: Vec::new(),
            body: None,
            timeout_seconds: 5,
            follow_redirects: false,
        }
    }

    #[tokio::test]
    async fn rechaza_esquemas_que_no_son_http() {
        // `file://` convertiría el cliente en un lector de archivos.
        for url in [
            "file:///C:/Windows/win.ini",
            "ftp://ejemplo.test/x",
            "data:text/plain,hola",
        ] {
            let error = send(&request(url)).await.unwrap_err().to_string();
            assert!(
                error.contains("http y https"),
                "{url} debería rechazarse: {error}"
            );
        }
    }

    #[tokio::test]
    async fn rechaza_una_direccion_mal_formada() {
        let error = send(&request("esto no es una url")).await.unwrap_err().to_string();
        assert!(error.contains("no es válida"), "{error}");
    }

    #[tokio::test]
    async fn rechaza_una_direccion_vacia() {
        let error = send(&request("   ")).await.unwrap_err().to_string();
        assert!(error.contains("escribe una dirección"), "{error}");
    }

    #[tokio::test]
    async fn rechaza_metodos_fuera_de_la_lista() {
        let mut peticion = request("https://ejemplo.test");
        peticion.method = "TRACE".into();

        let error = send(&peticion).await.unwrap_err().to_string();
        assert!(error.contains("no admitido"), "{error}");
    }

    #[test]
    fn la_lista_de_metodos_cubre_lo_habitual_y_excluye_lo_raro() {
        assert!(METHODS.contains(&"GET"));
        assert!(METHODS.contains(&"POST"));
        assert!(METHODS.contains(&"PATCH"));
        assert!(!METHODS.contains(&"CONNECT"));
        assert!(!METHODS.contains(&"TRACE"));
    }

    #[tokio::test]
    #[ignore = "necesita salida a la red"]
    async fn hace_una_peticion_de_verdad() {
        let response = send(&request("https://api.github.com")).await.unwrap();

        assert_eq!(response.status, 200);
        assert!(response.elapsed_ms > 0);
        assert!(!response.body.is_empty());
        assert!(response
            .content_type
            .as_deref()
            .unwrap_or_default()
            .contains("json"));
    }

    #[tokio::test]
    #[ignore = "necesita salida a la red"]
    async fn no_sigue_las_redirecciones_por_defecto() {
        // github.com/jgm redirige; sin seguirla, debe verse el 301 y el destino.
        let response = send(&request("http://github.com")).await.unwrap();

        assert!(response.status >= 300 && response.status < 400, "{}", response.status);
        assert!(response.redirect_to.is_some());
    }
}
