//! Comandos de conversión de formatos.
//!
//! Las rutas de archivo vienen de la interfaz, que solo las obtiene del diálogo
//! del sistema o de arrastrar y soltar: el usuario elige siempre qué se lee y
//! qué se escribe. Lo que sí se controla aquí es no pisar nada sin permiso
//! —escribir requiere `overwrite` explícito— y no dejar a medias un archivo si
//! la conversión falla a mitad.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::convert::{self, Format, Opts};
use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub extensions: &'static [&'static str],
    pub default_extension: &'static str,
    pub binary: bool,
    pub tabular: bool,
}

#[tauri::command]
pub fn list_formats() -> Vec<FormatInfo> {
    Format::ALL
        .iter()
        .map(|format| FormatInfo {
            id: format.id(),
            label: format.label(),
            extensions: format.extensions(),
            default_extension: format.default_extension(),
            binary: format.is_binary(),
            tabular: format.is_tabular(),
        })
        .collect()
}

/// Formatos de destino alcanzables desde uno de origen.
#[tauri::command]
pub fn convert_targets(from: String) -> AppResult<Vec<&'static str>> {
    let from = parse_format(&from)?;
    Ok(convert::available_targets(from)
        .into_iter()
        .map(Format::id)
        .collect())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextRequest {
    pub from: String,
    pub to: String,
    pub input: String,
    #[serde(default)]
    pub opts: Opts,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextResponse {
    pub output: String,
    pub bytes: usize,
}

/// Conversión del panel de texto.
#[tauri::command]
pub fn convert_text(request: TextRequest) -> AppResult<TextResponse> {
    let from = parse_format(&request.from)?;
    let to = parse_format(&request.to)?;

    if to.is_binary() {
        return Err(AppError::msg(format!(
            "{} es un formato binario: conviértelo a un archivo en vez de a texto.",
            to.label()
        )));
    }

    let bytes = convert::convert(from, to, request.input.as_bytes(), &request.opts)?;
    let bytes_len = bytes.len();
    let output = String::from_utf8(bytes)
        .map_err(|_| AppError::msg("la conversión produjo datos que no son texto"))?;

    Ok(TextResponse {
        output,
        bytes: bytes_len,
    })
}

/// Deduce el formato de un texto pegado o de una ruta.
#[tauri::command]
pub fn detect_format(path: Option<String>, sample: Option<String>) -> Option<&'static str> {
    let path = path.map(PathBuf::from);
    let sample = sample.unwrap_or_default();
    Format::detect(sample.as_bytes(), path.as_deref()).map(Format::id)
}

/// Por encima de esto el editor de la interfaz deja de responder, así que es
/// mejor negarse que congelar la ventana. Los archivos grandes se convierten
/// por el panel de lotes, que no los carga en memoria de la UI.
const MAX_EDITOR_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedFile {
    pub text: String,
    pub format: Option<&'static str>,
    pub path: String,
}

/// Carga un archivo de texto en el editor.
#[tauri::command]
pub fn load_text_file(path: String) -> AppResult<LoadedFile> {
    let source = PathBuf::from(&path);

    let size = std::fs::metadata(&source)
        .map_err(|e| AppError::msg(format!("no se pudo abrir «{path}»: {e}")))?
        .len();
    if size > MAX_EDITOR_BYTES {
        return Err(AppError::msg(format!(
            "«{path}» ocupa {:.1} MB y el editor admite hasta {} MB. \
             Usa la conversión por lotes.",
            size as f64 / 1_048_576.0,
            MAX_EDITOR_BYTES / 1_048_576
        )));
    }

    let bytes = std::fs::read(&source)?;
    let format = Format::detect(&bytes, Some(&source));

    if format.is_some_and(Format::is_binary) {
        return Err(AppError::msg(format!(
            "«{path}» es un archivo binario: conviértelo desde el panel de lotes."
        )));
    }

    let text = String::from_utf8(bytes)
        .map_err(|_| AppError::msg(format!("«{path}» no es texto UTF-8")))?;

    Ok(LoadedFile {
        text: text.strip_prefix('\u{feff}').unwrap_or(&text).to_string(),
        format: format.map(Format::id),
        path,
    })
}

/// Guarda el contenido del editor.
#[tauri::command]
pub fn save_text_file(path: String, contents: String, overwrite: bool) -> AppResult<()> {
    let destination = PathBuf::from(&path);
    if destination.exists() && !overwrite {
        return Err(AppError::msg(format!("«{path}» ya existe")));
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&destination, contents)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilesRequest {
    pub paths: Vec<String>,
    pub to: String,
    /// Carpeta de salida; si falta, junto a cada archivo de origen.
    pub output_dir: Option<String>,
    #[serde(default)]
    pub overwrite: bool,
    #[serde(default)]
    pub opts: Opts,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileOutcome {
    pub source: String,
    pub output: Option<String>,
    pub error: Option<String>,
}

/// Convierte archivos en lote.
///
/// Un archivo que falle no detiene al resto: cada uno informa de su resultado
/// por separado, porque en un lote de cien lo útil es saber cuáles fueron mal,
/// no perder los noventa y nueve que fueron bien.
#[tauri::command]
pub fn convert_files(request: FilesRequest) -> AppResult<Vec<FileOutcome>> {
    let to = parse_format(&request.to)?;
    let output_dir = request.output_dir.as_ref().map(PathBuf::from);

    let outcomes = request
        .paths
        .iter()
        .map(|source| {
            let source_path = PathBuf::from(source);
            match convert_one(
                &source_path,
                to,
                output_dir.as_deref(),
                request.overwrite,
                &request.opts,
            ) {
                Ok(destination) => FileOutcome {
                    source: source.clone(),
                    output: Some(destination.display().to_string()),
                    error: None,
                },
                Err(error) => FileOutcome {
                    source: source.clone(),
                    output: None,
                    error: Some(error.to_string()),
                },
            }
        })
        .collect();

    Ok(outcomes)
}

fn convert_one(
    source: &Path,
    to: Format,
    output_dir: Option<&Path>,
    overwrite: bool,
    opts: &Opts,
) -> AppResult<PathBuf> {
    let bytes = std::fs::read(source)
        .map_err(|e| AppError::msg(format!("no se pudo leer «{}»: {e}", source.display())))?;

    let from = Format::detect(&bytes, Some(source)).ok_or_else(|| {
        AppError::msg(format!(
            "no se reconoce el formato de «{}»",
            source.display()
        ))
    })?;

    let destination = destination_for(source, to, output_dir)?;
    if destination == source {
        return Err(AppError::msg(
            "el destino coincide con el origen: elige otra carpeta o formato",
        ));
    }
    if destination.exists() && !overwrite {
        return Err(AppError::msg(format!(
            "«{}» ya existe",
            destination.display()
        )));
    }

    let converted = convert::convert(from, to, &bytes, opts)?;

    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Se escribe en un temporal y se renombra: si el proceso muere a medias, el
    // usuario no se queda con un archivo truncado que parece bueno.
    let staging = destination.with_extension(format!("{}.parcial", to.default_extension()));
    std::fs::write(&staging, converted)?;
    std::fs::rename(&staging, &destination).inspect_err(|_| {
        let _ = std::fs::remove_file(&staging);
    })?;

    Ok(destination)
}

fn destination_for(source: &Path, to: Format, output_dir: Option<&Path>) -> AppResult<PathBuf> {
    let stem = source
        .file_stem()
        .ok_or_else(|| AppError::msg(format!("«{}» no tiene nombre", source.display())))?;

    let directory = match output_dir {
        Some(dir) => dir.to_path_buf(),
        None => source
            .parent()
            .ok_or_else(|| AppError::msg("el archivo de origen no está en ninguna carpeta"))?
            .to_path_buf(),
    };

    // Se compone el nombre a mano en vez de usar `set_extension`, que sustituye
    // desde el último punto y convertiría «informe.2026.q1.csv» en
    // «informe.2026.json», perdiendo parte del nombre.
    let file_name = format!("{}.{}", stem.to_string_lossy(), to.default_extension());
    Ok(directory.join(file_name))
}

fn parse_format(id: &str) -> AppResult<Format> {
    Format::from_id(id).ok_or_else(|| AppError::msg(format!("formato desconocido: «{id}»")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_destino_cambia_la_extension_y_conserva_el_nombre() {
        let source = Path::new("C:/datos/informe.csv");
        let destination = destination_for(source, Format::Json, None).unwrap();
        assert_eq!(destination, PathBuf::from("C:/datos/informe.json"));
    }

    #[test]
    fn el_destino_respeta_la_carpeta_de_salida() {
        let source = Path::new("C:/datos/informe.csv");
        let salida = Path::new("D:/exportado");
        let destination = destination_for(source, Format::Yaml, Some(salida)).unwrap();
        assert_eq!(destination, PathBuf::from("D:/exportado/informe.yaml"));
    }

    #[test]
    fn el_destino_conserva_los_puntos_del_nombre() {
        let source = Path::new("C:/datos/informe.2026.q1.csv");
        let destination = destination_for(source, Format::Json, None).unwrap();
        assert_eq!(destination, PathBuf::from("C:/datos/informe.2026.q1.json"));
    }

    #[test]
    fn convertir_a_un_formato_binario_desde_texto_se_rechaza() {
        let error = convert_text(TextRequest {
            from: "json".into(),
            to: "xlsx".into(),
            input: "[]".into(),
            opts: Opts::default(),
        })
        .unwrap_err()
        .to_string();

        assert!(error.contains("binario"), "{error}");
    }

    #[test]
    fn un_formato_inventado_da_error_claro() {
        let error = parse_format("docx").unwrap_err().to_string();
        assert!(error.contains("docx"), "{error}");
    }

    #[test]
    fn convierte_texto_entre_formatos() {
        let response = convert_text(TextRequest {
            from: "csv".into(),
            to: "json".into(),
            input: "a,b\n1,2\n".into(),
            opts: Opts {
                pretty: false,
                ..Opts::default()
            },
        })
        .unwrap();

        assert_eq!(response.output, r#"[{"a":1,"b":2}]"#);
        assert_eq!(response.bytes, response.output.len());
    }

    #[test]
    fn detecta_por_ruta_y_por_contenido() {
        assert_eq!(
            detect_format(Some("x/datos.yml".into()), None),
            Some("yaml")
        );
        assert_eq!(detect_format(None, Some("{\"a\":1}".into())), Some("json"));
        assert_eq!(detect_format(None, Some("a,b".into())), None);
    }

    #[test]
    fn convierte_un_archivo_real_y_no_pisa_lo_existente() {
        let dir = std::env::temp_dir().join(format!("ft-convert-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("datos.csv");
        std::fs::write(&source, "a,b\n1,2\n").unwrap();

        let destination =
            convert_one(&source, Format::Json, None, false, &Opts::default()).unwrap();
        assert_eq!(destination, dir.join("datos.json"));
        assert!(std::fs::read_to_string(&destination).unwrap().contains("\"a\""));

        // El segundo intento debe negarse en vez de sobrescribir.
        let error = convert_one(&source, Format::Json, None, false, &Opts::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("ya existe"), "{error}");

        // Con permiso explícito sí.
        convert_one(&source, Format::Json, None, true, &Opts::default()).unwrap();

        // Y no quedan archivos parciales por el camino.
        let parciales: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains("parcial"))
            .collect();
        assert!(parciales.is_empty(), "quedaron temporales: {parciales:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn el_lote_informa_de_cada_archivo_por_separado() {
        let dir = std::env::temp_dir().join(format!("ft-lote-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let bueno = dir.join("bueno.csv");
        std::fs::write(&bueno, "a\n1\n").unwrap();
        let roto = dir.join("roto.json");
        std::fs::write(&roto, "{esto no es json").unwrap();

        let outcomes = convert_files(FilesRequest {
            paths: vec![
                bueno.display().to_string(),
                roto.display().to_string(),
                dir.join("no-existe.csv").display().to_string(),
            ],
            to: "yaml".into(),
            output_dir: None,
            overwrite: true,
            opts: Opts::default(),
        })
        .unwrap();

        assert_eq!(outcomes.len(), 3);
        assert!(outcomes[0].output.is_some(), "{:?}", outcomes[0]);
        assert!(outcomes[1].error.is_some(), "un JSON roto debe fallar");
        assert!(outcomes[2].error.is_some(), "un archivo ausente debe fallar");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
