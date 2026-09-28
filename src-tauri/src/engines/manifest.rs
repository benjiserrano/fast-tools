//! Catálogo de motores externos, **fijado en el código**.
//!
//! Que el manifiesto esté compilado dentro del binario y no se descargue es la
//! primera de las reglas de seguridad de esta parte: si la lista de URL y
//! hashes viniera de la red, quien controlase ese servidor podría hacer que
//! fast-tools descargase y ejecutase cualquier cosa. Actualizar un motor exige
//! publicar una versión nueva de la aplicación, que es exactamente la barrera
//! que se quiere.
//!
//! Los hashes son los que publica cada proyecto para esa versión exacta. Se han
//! tomado de la API de GitHub, que los calcula sobre el archivo almacenado, que
//! es el mismo que descargará el usuario. Las versiones apuntan a etiquetas
//! inmutables a propósito: una etiqueta rodante como «latest» cambiaría de
//! contenido y el hash dejaría de cuadrar al día siguiente.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EngineId {
    Pandoc,
    Ffmpeg,
    LibreOffice,
}

impl EngineId {
    pub const ALL: &'static [EngineId] =
        &[EngineId::Pandoc, EngineId::Ffmpeg, EngineId::LibreOffice];

    pub fn id(self) -> &'static str {
        match self {
            EngineId::Pandoc => "pandoc",
            EngineId::Ffmpeg => "ffmpeg",
            EngineId::LibreOffice => "libreoffice",
        }
    }

    pub fn from_id(id: &str) -> Option<EngineId> {
        EngineId::ALL
            .iter()
            .copied()
            .find(|engine| engine.id().eq_ignore_ascii_case(id))
    }
}

/// De dónde se baja un motor y cómo comprobar que es el que se esperaba.
#[derive(Debug, Clone)]
pub struct Download {
    pub version: &'static str,
    pub url: &'static str,
    /// SHA-256 en minúsculas del archivo tal cual se descarga.
    pub sha256: &'static str,
    pub size_bytes: u64,
    /// Ruta del ejecutable dentro del archivo comprimido, con «/».
    pub executable_in_archive: &'static str,
    pub license: &'static str,
}

#[derive(Debug, Clone)]
pub struct EngineSpec {
    pub id: EngineId,
    pub name: &'static str,
    /// Nombre del ejecutable a buscar en el PATH del sistema.
    pub command: &'static str,
    pub description: &'static str,
    /// Qué desbloquea tenerlo instalado.
    pub enables: &'static str,
    /// `None` cuando el motor no se puede descargar y solo se detecta.
    pub download: Option<Download>,
    /// Por qué no se descarga, cuando no se descarga.
    pub no_download_reason: Option<&'static str>,
    /// Rutas de instalación habituales en Windows, para detectarlo sin PATH.
    pub system_paths: &'static [&'static str],
}

/// Anfitriones desde los que se acepta descargar.
///
/// Se comprueba en tiempo de ejecución además de tener las URL fijadas: si una
/// futura edición del manifiesto colara un dominio distinto por descuido, la
/// descarga se niega igualmente.
pub const ALLOWED_HOSTS: &[&str] = &["github.com", "objects.githubusercontent.com"];

pub const ENGINES: &[EngineSpec] = &[
    EngineSpec {
        id: EngineId::Pandoc,
        name: "Pandoc",
        command: "pandoc",
        description: "Conversor universal de documentos.",
        enables: "Markdown, HTML, DOCX, ODT, RST, LaTeX, EPUB y man.",
        download: Some(Download {
            version: "3.11",
            url: "https://github.com/jgm/pandoc/releases/download/3.11/pandoc-3.11-windows-x86_64.zip",
            sha256: "2ab72baf2399450e148ddf7a2a8689806c42e1bba71862b57e220fd9b8456d3d",
            size_bytes: 41_761_100,
            executable_in_archive: "pandoc-3.11/pandoc.exe",
            license: "GPL-2.0-or-later",
        }),
        no_download_reason: None,
        system_paths: &[],
    },
    EngineSpec {
        id: EngineId::Ffmpeg,
        name: "FFmpeg",
        command: "ffmpeg",
        description: "Conversor de audio y vídeo.",
        enables: "MP4, MKV, WebM, MOV, MP3, WAV, FLAC, OGG y AAC.",
        download: Some(Download {
            version: "8.1.3",
            url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-09-21-13-55/ffmpeg-n8.1.3-win64-gpl-8.1.zip",
            sha256: "c32c05e047d0bf2fef6eaee33c8ff3bc21ee5949394996e719c7d3a01630be0c",
            size_bytes: 191_997_859,
            executable_in_archive: "ffmpeg-n8.1.3-win64-gpl-8.1/bin/ffmpeg.exe",
            license: "GPL-3.0-or-later",
        }),
        no_download_reason: None,
        system_paths: &[],
    },
    EngineSpec {
        id: EngineId::LibreOffice,
        name: "LibreOffice",
        command: "soffice",
        description: "Suite ofimática, usada solo para exportar a PDF.",
        enables: "DOCX, ODT, XLSX y PPTX a PDF conservando el formato.",
        // LibreOffice se distribuye como instalador MSI de más de 350 MB, no
        // como archivo portable. Descargarlo y ejecutar un instalador del
        // sistema es otra clase de operación —toca el registro, pide permisos
        // de administrador— y no es lo que promete una aplicación portable.
        download: None,
        no_download_reason: Some(
            "LibreOffice solo se distribuye como instalador del sistema, no como \
             archivo portable. Instálalo aparte y fast-tools lo detectará, o \
             copia una versión portable en la carpeta de motores.",
        ),
        system_paths: &[
            r"C:\Program Files\LibreOffice\program\soffice.exe",
            r"C:\Program Files (x86)\LibreOffice\program\soffice.exe",
        ],
    },
];

pub fn spec(id: EngineId) -> &'static EngineSpec {
    ENGINES
        .iter()
        .find(|engine| engine.id == id)
        .expect("todo EngineId tiene entrada en ENGINES")
}

/// Comprueba que una URL apunta a un anfitrión permitido y usa HTTPS.
pub fn is_allowed_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host = rest
        .split('/')
        .next()
        .unwrap_or_default()
        .split('@')
        .next_back()
        .unwrap_or_default()
        .split(':')
        .next()
        .unwrap_or_default();

    ALLOWED_HOSTS.contains(&host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_identificador_tiene_ficha() {
        for id in EngineId::ALL {
            let entry = spec(*id);
            assert_eq!(entry.id, *id);
            assert!(!entry.name.is_empty());
            assert!(!entry.command.is_empty());
            assert!(!entry.enables.is_empty());
        }
    }

    #[test]
    fn los_identificadores_redondean() {
        for id in EngineId::ALL {
            assert_eq!(EngineId::from_id(id.id()), Some(*id));
        }
        assert!(EngineId::from_id("imagemagick").is_none());
    }

    #[test]
    fn toda_descarga_lleva_hash_de_64_caracteres_en_minusculas() {
        for entry in ENGINES {
            let Some(download) = &entry.download else {
                continue;
            };
            assert_eq!(
                download.sha256.len(),
                64,
                "{}: un SHA-256 son 64 caracteres",
                entry.name
            );
            assert!(
                download
                    .sha256
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "{}: el hash debe ir en hexadecimal minúscula",
                entry.name
            );
            assert!(download.size_bytes > 0, "{}: falta el tamaño", entry.name);
        }
    }

    #[test]
    fn toda_descarga_apunta_a_un_anfitrion_permitido() {
        for entry in ENGINES {
            if let Some(download) = &entry.download {
                assert!(
                    is_allowed_url(download.url),
                    "{} apunta a un anfitrión no permitido: {}",
                    entry.name,
                    download.url
                );
            }
        }
    }

    #[test]
    fn ninguna_descarga_apunta_a_una_etiqueta_rodante() {
        // Una etiqueta como «latest» cambia de contenido y dejaría el hash
        // obsoleto en cuanto el proyecto publicase una versión nueva.
        for entry in ENGINES {
            if let Some(download) = &entry.download {
                assert!(
                    !download.url.contains("/latest/"),
                    "{} usa una etiqueta rodante: {}",
                    entry.name,
                    download.url
                );
            }
        }
    }

    #[test]
    fn un_motor_sin_descarga_explica_por_que() {
        for entry in ENGINES {
            if entry.download.is_none() {
                assert!(
                    entry.no_download_reason.is_some(),
                    "{} no se descarga y no dice por qué",
                    entry.name
                );
            }
        }
    }

    #[test]
    fn la_lista_de_anfitriones_rechaza_lo_que_debe() {
        assert!(is_allowed_url("https://github.com/jgm/pandoc/releases/x.zip"));
        assert!(is_allowed_url("https://objects.githubusercontent.com/x"));

        // Sin cifrar.
        assert!(!is_allowed_url("http://github.com/x"));
        // Otro dominio.
        assert!(!is_allowed_url("https://evil.test/x"));
        // Subdominio que solo empieza igual.
        assert!(!is_allowed_url("https://github.com.evil.test/x"));
        // Credenciales en la URL para colar otro anfitrión.
        assert!(!is_allowed_url("https://github.com@evil.test/x"));
        // Dominio que termina igual.
        assert!(!is_allowed_url("https://notgithub.com/x"));
    }
}
