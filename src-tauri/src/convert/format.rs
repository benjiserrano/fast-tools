//! Formatos de datos estructurados y cómo se reconocen.

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Json,
    Yaml,
    Toml,
    Xml,
    Csv,
    Tsv,
    Xlsx,
}

impl Format {
    pub const ALL: &'static [Format] = &[
        Format::Json,
        Format::Yaml,
        Format::Toml,
        Format::Xml,
        Format::Csv,
        Format::Tsv,
        Format::Xlsx,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::Yaml => "yaml",
            Format::Toml => "toml",
            Format::Xml => "xml",
            Format::Csv => "csv",
            Format::Tsv => "tsv",
            Format::Xlsx => "xlsx",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Format::Json => "JSON",
            Format::Yaml => "YAML",
            Format::Toml => "TOML",
            Format::Xml => "XML",
            Format::Csv => "CSV",
            Format::Tsv => "TSV",
            Format::Xlsx => "Excel (XLSX)",
        }
    }

    /// Extensiones aceptadas al detectar. La primera es la que se usa al escribir.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Format::Json => &["json", "jsonc"],
            Format::Yaml => &["yaml", "yml"],
            Format::Toml => &["toml"],
            Format::Xml => &["xml"],
            Format::Csv => &["csv"],
            Format::Tsv => &["tsv", "tab"],
            Format::Xlsx => &["xlsx", "xlsm"],
        }
    }

    pub fn default_extension(self) -> &'static str {
        self.extensions()[0]
    }

    /// Si el formato no se puede mostrar ni editar como texto.
    pub fn is_binary(self) -> bool {
        matches!(self, Format::Xlsx)
    }

    /// Si el formato es una tabla plana: al escribir hay que aplanar lo anidado.
    pub fn is_tabular(self) -> bool {
        matches!(self, Format::Csv | Format::Tsv | Format::Xlsx)
    }

    /// Separador de columnas de los formatos delimitados.
    pub fn delimiter(self) -> Option<u8> {
        match self {
            Format::Csv => Some(b','),
            Format::Tsv => Some(b'\t'),
            _ => None,
        }
    }

    pub fn from_id(id: &str) -> Option<Format> {
        Format::ALL
            .iter()
            .copied()
            .find(|f| f.id().eq_ignore_ascii_case(id))
    }

    pub fn from_extension(ext: &str) -> Option<Format> {
        let ext = ext.trim_start_matches('.').to_ascii_lowercase();
        Format::ALL
            .iter()
            .copied()
            .find(|f| f.extensions().contains(&ext.as_str()))
    }

    pub fn from_path(path: &Path) -> Option<Format> {
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(Format::from_extension)
    }

    /// Adivina el formato a partir del nombre y, si no basta, del contenido.
    ///
    /// La extensión manda: es la intención declarada del usuario. El contenido
    /// solo decide cuando no hay extensión o no la reconocemos, y entonces se
    /// limita a lo que se puede afirmar sin ambigüedad. CSV y TSV no se
    /// adivinan por contenido porque cualquier texto con comas lo parecería.
    pub fn detect(bytes: &[u8], path: Option<&Path>) -> Option<Format> {
        if let Some(format) = path.and_then(Format::from_path) {
            return Some(format);
        }
        Format::sniff(bytes)
    }

    fn sniff(bytes: &[u8]) -> Option<Format> {
        // XLSX es un ZIP: la firma es inequívoca.
        if bytes.starts_with(b"PK\x03\x04") {
            return Some(Format::Xlsx);
        }

        let text = std::str::from_utf8(bytes).ok()?;
        let head = text.trim_start_matches('\u{feff}').trim_start();

        match head.as_bytes().first()? {
            b'<' => Some(Format::Xml),
            b'{' | b'[' => Some(Format::Json),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn los_identificadores_son_unicos_y_redondean() {
        for format in Format::ALL {
            assert_eq!(Format::from_id(format.id()), Some(*format));
        }
    }

    #[test]
    fn cada_extension_apunta_a_un_solo_formato() {
        let mut vistas = Vec::new();
        for format in Format::ALL {
            for ext in format.extensions() {
                assert!(!vistas.contains(ext), "extensión duplicada: {ext}");
                vistas.push(ext);
            }
        }
    }

    #[test]
    fn detecta_por_extension_sin_mirar_el_contenido() {
        let path = PathBuf::from("datos.YAML");
        assert_eq!(Format::detect(b"a: 1", Some(&path)), Some(Format::Yaml));

        // La extensión gana aunque el contenido parezca otra cosa.
        let path = PathBuf::from("datos.csv");
        assert_eq!(Format::detect(b"{\"a\":1}", Some(&path)), Some(Format::Csv));
    }

    #[test]
    fn detecta_xlsx_por_su_firma_zip() {
        assert_eq!(Format::sniff(b"PK\x03\x04resto"), Some(Format::Xlsx));
    }

    #[test]
    fn detecta_json_y_xml_por_contenido() {
        assert_eq!(Format::sniff(b"  \n{\"a\": 1}"), Some(Format::Json));
        assert_eq!(Format::sniff(b"[1, 2]"), Some(Format::Json));
        assert_eq!(Format::sniff(b"<?xml version=\"1.0\"?><a/>"), Some(Format::Xml));
        // Con marca de orden de bytes por delante.
        assert_eq!(Format::sniff("\u{feff}{\"a\":1}".as_bytes()), Some(Format::Json));
    }

    #[test]
    fn no_adivina_lo_ambiguo() {
        // Podría ser CSV, TSV o texto suelto: mejor no elegir por el usuario.
        assert_eq!(Format::sniff(b"nombre,edad\nana,33"), None);
        assert_eq!(Format::sniff(b"clave: valor"), None);
        assert_eq!(Format::detect(b"nombre,edad", None), None);
    }

    #[test]
    fn los_delimitados_tienen_separador_y_los_demas_no() {
        assert_eq!(Format::Csv.delimiter(), Some(b','));
        assert_eq!(Format::Tsv.delimiter(), Some(b'\t'));
        assert_eq!(Format::Json.delimiter(), None);
    }
}
