//! Catálogo de herramientas.
//!
//! Fuente única de verdad: la UI construye la barra lateral y la paleta Ctrl+K
//! a partir de esta lista, no de una copia en TypeScript. Añadir una herramienta
//! es añadir una fila aquí más su panel en `src/tools/<id>/`.
//!
//! Ahora mismo todas están implementadas. `Status::Planned` se mantiene para
//! las que se añadan en el futuro: una herramienta marcada así aparece en la
//! paleta con su número de fase en lugar de esconderse, de forma que la hoja de
//! ruta es visible y nadie se encuentra un hueco sin explicación.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    Convert,
    Text,
    Crypto,
    Net,
    Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Runtime {
    /// Transformación pura de string: se ejecuta en el WebView, sin IPC.
    Web,
    /// Toca disco, criptografía, imágenes o red: se ejecuta en Rust.
    Native,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Ready,
    // Sin usar mientras el catálogo esté completo. Se conserva porque es el
    // mecanismo con el que se anuncian las herramientas de fases futuras.
    #[allow(dead_code)]
    Planned,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub id: &'static str,
    pub name: &'static str,
    pub summary: &'static str,
    pub category: Category,
    pub runtime: Runtime,
    pub status: Status,
    /// Fase del plan en la que se implementa. Solo informativo para la UI.
    pub phase: u8,
    /// Términos alternativos para la búsqueda de la paleta.
    pub keywords: &'static [&'static str],
}

/// Azúcar sintáctico para que el catálogo se lea como una tabla.
/// Formato: `id nombre [Categoría Runtime Estado fase] resumen { palabras clave }`
macro_rules! catalog {
    ($(
        $id:literal $name:literal [$cat:ident $rt:ident $status:ident $phase:literal]
        $summary:literal { $($kw:literal),* }
    )*) => {
        &[$(
            Tool {
                id: $id,
                name: $name,
                summary: $summary,
                category: Category::$cat,
                runtime: Runtime::$rt,
                status: Status::$status,
                phase: $phase,
                keywords: &[$($kw),*],
            }
        ),*]
    };
}

pub const TOOLS: &[Tool] = catalog! {
    // ── Conversión de archivos ────────────────────────────────────────────
    "convert-data" "Datos estructurados" [Convert Native Ready 1]
        "CSV, JSON, YAML, TOML, XML y XLSX en cualquier dirección"
        { "csv", "json", "yaml", "toml", "xml", "xlsx", "excel", "convertir" }

    "convert-doc" "Documentos" [Convert Native Ready 4]
        "Markdown, HTML, PDF y DOCX"
        { "markdown", "md", "html", "pdf", "docx", "word", "documento" }

    "convert-image" "Formato de imagen" [Convert Native Ready 3]
        "PNG, JPG, WebP, AVIF, GIF, BMP, TIFF, ICO y SVG"
        { "png", "jpg", "jpeg", "webp", "avif", "gif", "svg", "ico", "imagen" }

    "convert-media" "Audio y vídeo" [Convert Native Ready 4]
        "MP4, MKV, WebM, MP3, WAV y FLAC"
        { "mp4", "mkv", "webm", "mp3", "wav", "flac", "video", "audio", "ffmpeg" }

    "engines" "Motores externos" [Convert Native Ready 4]
        "Gestiona Pandoc, FFmpeg y LibreOffice: instalar, verificar y quitar"
        { "motor", "pandoc", "ffmpeg", "libreoffice", "instalar", "descargar" }

    "convert-batch" "Conversión por lotes" [Convert Native Ready 1]
        "Arrastra una carpeta y convierte todo su contenido"
        { "lote", "batch", "masivo", "carpeta" }

    // ── Texto y datos ─────────────────────────────────────────────────────
    "json-format" "JSON" [Text Web Ready 2]
        "Formatear, validar y minificar"
        { "json", "pretty", "formatear", "validar", "minificar" }

    "yaml-format" "YAML" [Text Native Ready 2]
        "Formatear y validar, con resolución de anclas"
        { "yaml", "yml", "formatear", "validar", "anclas" }

    "xml-format" "XML" [Text Native Ready 2]
        "Formatear y validar"
        { "xml", "formatear", "validar" }

    "sql-format" "SQL" [Text Native Ready 2]
        "Formatear consultas con indentación consistente"
        { "sql", "consulta", "query", "formatear" }

    "text-diff" "Diff de texto" [Text Native Ready 2]
        "Comparación lado a lado con resaltado por palabra"
        { "diff", "comparar", "diferencias", "cambios" }

    "list-diff" "List diff" [Text Web Ready 2]
        "Compara dos listas y muestra exclusivos, elementos comunes y la union"
        { "list diff", "lista", "listas", "interseccion", "union", "comun" }

    "base64" "Base64" [Text Web Ready 2]
        "Codificar y decodificar texto o archivos"
        { "base64", "b64", "codificar", "decodificar" }

    "url-encode" "URL encode" [Text Web Ready 2]
        "Percent-encoding de componentes y cadenas de consulta"
        { "url", "uri", "percent", "encode", "escapar" }

    "escape" "Escapado" [Text Web Ready 2]
        "HTML, JSON, regex y secuencias de shell"
        { "escape", "escapar", "html", "entidades" }

    "case-convert" "Mayúsculas y minúsculas" [Text Web Ready 2]
        "camelCase, snake_case, kebab-case, PascalCase, CONSTANT_CASE"
        { "camel", "snake", "kebab", "pascal", "constante", "case" }

    "text-stats" "Estadísticas de texto" [Text Web Ready 2]
        "Líneas, palabras, caracteres y bytes"
        { "contar", "lineas", "palabras", "caracteres", "bytes" }

    "line-ops" "Operaciones por línea" [Text Web Ready 2]
        "Ordenar, invertir, deduplicar, numerar y filtrar"
        { "ordenar", "sort", "duplicados", "unique", "filtrar" }

    "lorem" "Lorem ipsum" [Text Web Ready 2]
        "Texto de relleno por palabras, frases o párrafos"
        { "lorem", "ipsum", "relleno", "placeholder", "dummy" }

    // ── Cripto e identificadores ──────────────────────────────────────────
    "hash-text" "Hash de texto" [Crypto Native Ready 2]
        "MD5, SHA-1, SHA-256, SHA-512, BLAKE3 y CRC32"
        { "hash", "md5", "sha1", "sha256", "sha512", "blake3", "crc32" }

    "hash-file" "Hash y checksum de archivo" [Crypto Native Ready 2]
        "Calcula y verifica sumas de comprobación"
        { "checksum", "verificar", "integridad", "archivo", "sha256sum" }

    "hmac" "HMAC" [Crypto Native Ready 2]
        "Código de autenticación de mensaje con clave"
        { "hmac", "firma", "clave", "autenticacion" }

    "uuid-gen" "UUID y ULID" [Crypto Native Ready 2]
        "Generación en lote de UUID v4/v7 y ULID"
        { "uuid", "guid", "ulid", "identificador", "v4", "v7" }

    "jwt-decode" "JWT" [Crypto Native Ready 2]
        "Decodifica, valida la firma y avisa de expiración"
        { "jwt", "token", "bearer", "jose", "claims" }

    "password-hash" "bcrypt y argon2" [Crypto Native Ready 2]
        "Generar y verificar hashes de contraseña"
        { "bcrypt", "argon2", "password", "contrasena", "verificar" }

    "password-gen" "Generador de contraseñas" [Crypto Native Ready 2]
        "Aleatorias, con estimación de entropía"
        { "password", "contrasena", "aleatorio", "entropia", "seguro" }

    "cert-inspect" "Certificados X.509" [Crypto Native Ready 2]
        "Emisor, sujeto, SAN, vigencia y huella digital"
        { "certificado", "x509", "pem", "tls", "ssl", "huella" }

    // ── Web y red ─────────────────────────────────────────────────────────
    "regex-test" "Probador de regex" [Net Native Ready 5]
        "Coincidencias en vivo, grupos de captura y explicación del patrón"
        { "regex", "regexp", "expresion", "patron", "captura" }

    "cron-parse" "Expresiones cron" [Net Native Ready 5]
        "Interpreta el patrón y muestra las próximas ejecuciones"
        { "cron", "crontab", "programar", "schedule" }

    "http-client" "Cliente HTTP" [Net Native Ready 5]
        "Peticiones con historial y colecciones guardadas"
        { "http", "rest", "api", "peticion", "curl", "postman" }

    "url-inspect" "Inspector de URL" [Net Web Ready 5]
        "Descompone y reconstruye una URL por partes"
        { "url", "uri", "query", "parametros", "host" }

    "timestamp" "Marcas de tiempo" [Net Web Ready 5]
        "Unix, ISO 8601, RFC 3339 y conversión de zona horaria"
        { "timestamp", "epoch", "unix", "iso8601", "fecha", "zona" }

    "http-status" "Códigos HTTP" [Net Web Ready 5]
        "Referencia de códigos de estado con su significado"
        { "http", "status", "404", "500", "codigo", "referencia" }

    // ── Imágenes y multimedia ─────────────────────────────────────────────
    "image-resize" "Redimensionar imagen" [Image Native Ready 3]
        "Escalar, recortar y rotar"
        { "redimensionar", "resize", "escalar", "recortar", "rotar" }

    "image-compress" "Comprimir imagen" [Image Native Ready 3]
        "Vista previa de calidad y peso resultante"
        { "comprimir", "optimizar", "calidad", "peso", "tamano" }

    "favicon-gen" "Generador de favicon" [Image Native Ready 3]
        "ICO multi-resolución desde PNG o SVG"
        { "favicon", "ico", "icono", "web" }

    "qr-gen" "Generador de QR" [Image Native Ready 3]
        "Códigos QR en PNG y SVG con nivel de corrección configurable"
        { "qr", "codigo", "barras", "escanear" }

    "exif-view" "Metadatos EXIF" [Image Native Ready 3]
        "Leer y eliminar metadatos de fotografías"
        { "exif", "metadatos", "gps", "camara", "limpiar" }

    "color-palette" "Paleta de colores" [Image Native Ready 3]
        "Extrae los colores dominantes de una imagen"
        { "paleta", "colores", "dominante", "extraer" }

    "color-convert" "Conversor de color" [Image Web Ready 3]
        "HEX, RGB, HSL y OKLCH con comprobación de contraste WCAG"
        { "color", "hex", "rgb", "hsl", "oklch", "contraste", "wcag" }
};

pub fn find(id: &str) -> Option<&'static Tool> {
    TOOLS.iter().find(|t| t.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn los_identificadores_son_unicos() {
        let mut vistos = HashSet::new();
        for tool in TOOLS {
            assert!(vistos.insert(tool.id), "identificador duplicado: {}", tool.id);
        }
    }

    #[test]
    fn los_identificadores_son_kebab_case() {
        // La UI los usa como nombre de carpeta y como fragmento de ruta.
        for tool in TOOLS {
            assert!(
                tool.id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "identificador no kebab-case: {}",
                tool.id
            );
        }
    }

    #[test]
    fn los_metadatos_estan_completos() {
        for tool in TOOLS {
            assert!(!tool.name.is_empty(), "{} sin nombre", tool.id);
            assert!(!tool.summary.is_empty(), "{} sin resumen", tool.id);
            assert!(!tool.keywords.is_empty(), "{} sin palabras clave", tool.id);
            assert!((1..=6).contains(&tool.phase), "{} con fase inválida", tool.id);
        }
    }

    #[test]
    fn find_localiza_y_descarta() {
        assert_eq!(find("hash-text").map(|t| t.id), Some("hash-text"));
        assert!(find("no-existe").is_none());
    }
}
