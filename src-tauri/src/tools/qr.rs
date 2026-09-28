//! Generación de códigos QR.

use fast_qr::convert::{image::ImageBuilder, svg::SvgBuilder, Builder, Shape};
use fast_qr::qr::QRBuilder;
use fast_qr::ECL;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// Un QR admite hasta 2 953 bytes en la versión 40 con corrección baja. Pasado
/// eso no hay código posible, y conviene decirlo antes de intentarlo.
const MAX_BYTES: usize = 2_953;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Correction {
    /// Recupera el 7 % del código. El más pequeño.
    Low,
    /// 15 %. Suficiente para pantalla.
    Medium,
    /// 25 %.
    Quartile,
    /// 30 %. Para impresión o si va a llevar un logotipo encima.
    High,
}

impl Correction {
    pub const ALL: &'static [Correction] = &[
        Correction::Low,
        Correction::Medium,
        Correction::Quartile,
        Correction::High,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Correction::Low => "low",
            Correction::Medium => "medium",
            Correction::Quartile => "quartile",
            Correction::High => "high",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Correction::Low => "Baja (7 %)",
            Correction::Medium => "Media (15 %)",
            Correction::Quartile => "Alta (25 %)",
            Correction::High => "Máxima (30 %)",
        }
    }

    pub fn note(self) -> &'static str {
        match self {
            Correction::Low => "El código más pequeño. Para pantallas limpias.",
            Correction::Medium => "Equilibrio habitual entre tamaño y tolerancia.",
            Correction::Quartile => "Aguanta suciedad o impresión de baja calidad.",
            Correction::High => {
                "Sigue leyéndose con un logotipo encima o parcialmente tapado."
            }
        }
    }

    fn to_ecl(self) -> ECL {
        match self {
            Correction::Low => ECL::L,
            Correction::Medium => ECL::M,
            Correction::Quartile => ECL::Q,
            Correction::High => ECL::H,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct QrOptions {
    pub correction: Correction,
    /// Lado del PNG en píxeles. No aplica al SVG, que es vectorial.
    pub size: u32,
    pub foreground: String,
    pub background: String,
    /// Margen en módulos. La especificación pide cuatro como mínimo: sin zona
    /// tranquila muchos lectores no encuentran el código.
    pub margin: u32,
}

impl Default for QrOptions {
    fn default() -> Self {
        QrOptions {
            correction: Correction::Medium,
            size: 512,
            foreground: "#000000".to_string(),
            background: "#ffffff".to_string(),
            margin: 4,
        }
    }
}

fn build(content: &str, options: &QrOptions) -> AppResult<fast_qr::QRCode> {
    if content.is_empty() {
        return Err(AppError::msg("escribe el contenido del código"));
    }
    if content.len() > MAX_BYTES {
        return Err(AppError::msg(format!(
            "un QR admite como mucho {MAX_BYTES} bytes y esto ocupa {}. \
             Acorta el contenido o enlaza a él en vez de incrustarlo.",
            content.len()
        )));
    }

    QRBuilder::new(content)
        .ecl(options.correction.to_ecl())
        .build()
        .map_err(|e| AppError::msg(format!("no se pudo construir el código: {e}")))
}

pub fn to_svg(content: &str, options: &QrOptions) -> AppResult<String> {
    let code = build(content, options)?;
    let [red, green, blue] = crate::tools::images::parse_color(&options.foreground)?;
    let [bg_red, bg_green, bg_blue] = crate::tools::images::parse_color(&options.background)?;

    Ok(SvgBuilder::default()
        .shape(Shape::Square)
        .margin(options.margin as usize)
        .module_color([red, green, blue, 255])
        .background_color([bg_red, bg_green, bg_blue, 255])
        .to_str(&code))
}

pub fn to_png(content: &str, options: &QrOptions) -> AppResult<Vec<u8>> {
    let code = build(content, options)?;
    let [red, green, blue] = crate::tools::images::parse_color(&options.foreground)?;
    let [bg_red, bg_green, bg_blue] = crate::tools::images::parse_color(&options.background)?;

    let size = options.size.clamp(64, 4096);

    ImageBuilder::default()
        .shape(Shape::Square)
        .margin(options.margin as usize)
        .module_color([red, green, blue, 255])
        .background_color([bg_red, bg_green, bg_blue, 255])
        .fit_width(size)
        .to_bytes(&code)
        .map_err(|e| AppError::msg(format!("no se pudo dibujar el código: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genera_svg_con_el_contenido_dibujado() {
        let svg = to_svg("https://fast-tools.test", &QrOptions::default()).unwrap();

        assert!(svg.contains("<svg"), "{}", &svg[..svg.len().min(80)]);
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn genera_png_del_tamano_pedido() {
        let options = QrOptions {
            size: 256,
            ..QrOptions::default()
        };
        let png = to_png("hola", &options).unwrap();

        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"), "debe ser un PNG");

        let decoded = crate::tools::images::load(&png, None).unwrap();
        assert_eq!(decoded.width(), 256);
        assert_eq!(decoded.height(), 256);
    }

    #[test]
    fn respeta_los_colores_elegidos() {
        let options = QrOptions {
            foreground: "#0000ff".to_string(),
            background: "#ffff00".to_string(),
            ..QrOptions::default()
        };
        let svg = to_svg("x", &options).unwrap();

        assert!(svg.to_lowercase().contains("0000ff"), "{svg:.200}");
    }

    #[test]
    fn mas_correccion_produce_un_codigo_mas_denso() {
        let contenido = "contenido de prueba para comparar densidades";

        let baja = to_svg(
            contenido,
            &QrOptions {
                correction: Correction::Low,
                ..QrOptions::default()
            },
        )
        .unwrap();
        let alta = to_svg(
            contenido,
            &QrOptions {
                correction: Correction::High,
                ..QrOptions::default()
            },
        )
        .unwrap();

        // Más corrección significa más módulos, y por tanto más marcado SVG.
        assert!(alta.len() > baja.len(), "{} vs {}", alta.len(), baja.len());
    }

    #[test]
    fn rechaza_el_contenido_vacio() {
        let error = to_svg("", &QrOptions::default()).unwrap_err().to_string();
        assert!(error.contains("contenido"), "{error}");
    }

    #[test]
    fn rechaza_contenido_que_no_cabe_explicando_el_limite() {
        let enorme = "a".repeat(MAX_BYTES + 1);
        let error = to_svg(&enorme, &QrOptions::default())
            .unwrap_err()
            .to_string();

        assert!(error.contains(&MAX_BYTES.to_string()), "{error}");
        assert!(error.contains("Acorta"), "{error}");
    }

    #[test]
    fn todos_los_niveles_tienen_etiqueta_y_nota() {
        for level in Correction::ALL {
            assert!(!level.id().is_empty());
            assert!(!level.label().is_empty());
            assert!(!level.note().is_empty());
        }
    }
}
