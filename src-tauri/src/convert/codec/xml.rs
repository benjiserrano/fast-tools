//! XML al pivote y de vuelta.
//!
//! XML no tiene un mapeo canónico a JSON, así que hace falta una convención.
//! Esta es la más extendida y la que usan la mayoría de las herramientas:
//!
//! - El documento produce `{ "<raíz>": contenido }`, conservando el nombre del
//!   elemento raíz para que la vuelta a XML lo recupere.
//! - Los atributos van con prefijo `@`: `<a id="1"/>` → `{"@id": 1}`.
//! - El texto de un elemento que además tiene atributos o hijos va en `#text`.
//! - Los hijos repetidos se agrupan en lista: `<i/><i/>` → `{"i": [ ... ]}`.
//!
//! Un elemento sin atributos ni hijos se reduce a su texto, porque envolver
//! `<nombre>Ana</nombre>` en `{"#text": "Ana"}` haría ilegible el JSON
//! resultante en el caso más común.

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use serde_json::{Map, Value};

use super::{as_text, Codec};
use crate::convert::table::{cell_to_string, infer_scalar};
use crate::convert::{ConvertError, ConvertResult, Format, Opts};

pub struct Xml;

const ATTRIBUTE_PREFIX: char = '@';
const TEXT_KEY: &str = "#text";

impl Codec for Xml {
    fn decode(&self, bytes: &[u8], opts: &Opts) -> ConvertResult<Value> {
        let node = parse(as_text(bytes)?)?;

        let mut root = Map::new();
        for (name, child) in node.children {
            insert_child(&mut root, name, node_to_value(child, opts));
        }
        Ok(Value::Object(root))
    }

    fn encode(&self, value: &Value, opts: &Opts) -> ConvertResult<Vec<u8>> {
        let indent = opts.effective_indent();
        let mut writer = if indent > 0 {
            Writer::new_with_indent(Vec::new(), b' ', indent)
        } else {
            Writer::new(Vec::new())
        };

        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .map_err(|e| ConvertError::encode(Format::Xml, e))?;

        let (root_name, content) = root_of(value, &opts.xml_root);
        write_element(&mut writer, &root_name, content, opts)?;

        Ok(writer.into_inner())
    }
}

// ── Lectura ───────────────────────────────────────────────────────────────

#[derive(Default)]
struct Node {
    attributes: Vec<(String, String)>,
    children: Vec<(String, Node)>,
    text: String,
}

fn parse(text: &str) -> ConvertResult<Node> {
    // Sin recorte automático: `trim_text` recorta cada fragmento por separado,
    // y el texto con entidades llega partido en varios eventos. Recortarlos uno
    // a uno convertiría «a &lt; b» en «ab». El recorte se hace una sola vez, ya
    // con el texto completo, en `node_to_value`.
    let mut reader = Reader::from_str(text);

    // El fondo de la pila es un nodo virtual que sostiene el elemento raíz.
    let mut stack: Vec<(String, Node)> = vec![(String::new(), Node::default())];

    loop {
        let event = reader
            .read_event()
            .map_err(|e| ConvertError::parse(Format::Xml, e))?;

        match event {
            Event::Start(start) => {
                let (name, node) = open(&start)?;
                stack.push((name, node));
            }
            Event::Empty(start) => {
                let (name, node) = open(&start)?;
                attach(&mut stack, name, node)?;
            }
            Event::End(end) => {
                // El fondo de la pila es el nodo virtual: si solo queda él, el
                // documento cierra una etiqueta que nunca se abrió.
                if stack.len() <= 1 {
                    return Err(closing_error(&end));
                }
                let (name, node) = stack.pop().expect("comprobado justo arriba");
                attach(&mut stack, name, node)?;
            }
            Event::Text(text) => {
                let raw = text.xml10_content();
                let value = quick_xml::escape::unescape(&raw)
                    .map_err(|e| ConvertError::parse(Format::Xml, e))?;
                push_text(&mut stack, &value);
            }
            // El contenido de una sección CDATA es literal: no lleva entidades
            // que deshacer.
            Event::CData(data) => {
                let value = data.into_inner();
                push_text(&mut stack, &value);
            }
            // quick-xml entrega cada referencia a entidad como evento propio,
            // así que el texto que las contiene llega troceado y hay que
            // resolverlas y volver a pegarlas.
            Event::GeneralRef(entity) => {
                let name = entity.into_inner();
                let reference = format!("&{name};");
                let resolved = quick_xml::escape::unescape(&reference).map_err(|e| {
                    ConvertError::parse(Format::Xml, format!("entidad «{reference}» no válida: {e}"))
                })?;
                push_text(&mut stack, &resolved);
            }
            Event::Eof => break,
            _ => {}
        }
    }

    let (_, root) = stack.pop().expect("el nodo virtual siempre está");
    if !stack.is_empty() {
        return Err(ConvertError::parse(
            Format::Xml,
            "hay elementos sin cerrar al final del documento",
        ));
    }
    if root.children.is_empty() {
        return Err(ConvertError::parse(
            Format::Xml,
            "el documento no tiene ningún elemento",
        ));
    }
    Ok(root)
}

fn open(start: &BytesStart) -> ConvertResult<(String, Node)> {
    let name = start.name().as_ref().to_string();

    let mut attributes = Vec::new();
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|e| ConvertError::parse(Format::Xml, e))?;
        let key = attribute.key.as_ref().to_string();
        // Normaliza además los saltos de línea dentro del valor, no solo las
        // entidades, que es lo que manda la especificación para atributos.
        let value = attribute
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .map_err(|e| ConvertError::parse(Format::Xml, e))?
            .into_owned();
        attributes.push((key, value));
    }

    Ok((
        name,
        Node {
            attributes,
            ..Node::default()
        },
    ))
}

fn attach(stack: &mut [(String, Node)], name: String, node: Node) -> ConvertResult<()> {
    let parent = stack.last_mut().ok_or_else(|| {
        ConvertError::parse(Format::Xml, "se cerró un elemento que nunca se abrió")
    })?;
    parent.1.children.push((name, node));
    Ok(())
}

fn push_text(stack: &mut [(String, Node)], value: &str) {
    if let Some(top) = stack.last_mut() {
        top.1.text.push_str(value);
    }
}

fn closing_error(end: &BytesEnd) -> ConvertError {
    ConvertError::parse(
        Format::Xml,
        format!(
            "se cerró «{}» sin que estuviera abierto",
            end.name().as_ref()
        ),
    )
}

fn node_to_value(node: Node, opts: &Opts) -> Value {
    // En XML el espacio alrededor del texto de un elemento es formato, no dato:
    // se recorta una sola vez aquí, con el texto ya completo.
    let scalar = |raw: &str| {
        let trimmed = raw.trim();
        if opts.infer_types {
            infer_scalar(trimmed)
        } else {
            Value::String(trimmed.to_string())
        }
    };

    if node.attributes.is_empty() && node.children.is_empty() {
        return scalar(&node.text);
    }

    let mut map = Map::new();
    for (key, value) in node.attributes {
        map.insert(format!("{ATTRIBUTE_PREFIX}{key}"), scalar(&value));
    }
    for (name, child) in node.children {
        insert_child(&mut map, name, node_to_value(child, opts));
    }
    if !node.text.trim().is_empty() {
        map.insert(TEXT_KEY.to_string(), scalar(&node.text));
    }

    Value::Object(map)
}

/// Inserta un hijo agrupando los nombres repetidos en una lista.
fn insert_child(map: &mut Map<String, Value>, name: String, value: Value) {
    match map.get_mut(&name) {
        Some(Value::Array(existing)) => existing.push(value),
        Some(slot) => {
            let previous = slot.take();
            *slot = Value::Array(vec![previous, value]);
        }
        None => {
            map.insert(name, value);
        }
    }
}

// ── Escritura ─────────────────────────────────────────────────────────────

/// Elige el elemento raíz.
///
/// Un objeto con una sola clave es la forma que produce [`Xml::decode`], así
/// que se reconoce para que la ida y vuelta conserve el nombre original. En
/// cualquier otro caso hace falta un nombre inventado, y ahí manda `xml_root`.
fn root_of<'a>(value: &'a Value, fallback: &str) -> (String, &'a Value) {
    if let Value::Object(map) = value {
        if map.len() == 1 {
            if let Some((name, inner)) = map.iter().next() {
                if is_valid_element_name(name) {
                    return (name.clone(), inner);
                }
            }
        }
    }
    (fallback.to_string(), value)
}

fn write_element<W: std::io::Write>(
    writer: &mut Writer<W>,
    name: &str,
    value: &Value,
    opts: &Opts,
) -> ConvertResult<()> {
    let fail = |e: std::io::Error| ConvertError::encode(Format::Xml, e);

    match value {
        // Un nombre que se repite es un elemento que se repite, no un elemento
        // que contiene una lista: es la inversa exacta de la lectura.
        Value::Array(items) => {
            for item in items {
                write_element(writer, name, item, opts)?;
            }
            Ok(())
        }
        Value::Object(map) => {
            let mut start = BytesStart::new(name);
            for (key, attribute) in map {
                if let Some(attribute_name) = key.strip_prefix(ATTRIBUTE_PREFIX) {
                    start.push_attribute((attribute_name, cell_to_string(attribute).as_str()));
                }
            }

            let text = map.get(TEXT_KEY);
            let children: Vec<(&String, &Value)> = map
                .iter()
                .filter(|(key, _)| !key.starts_with(ATTRIBUTE_PREFIX) && key.as_str() != TEXT_KEY)
                .collect();

            if text.is_none() && children.is_empty() {
                writer.write_event(Event::Empty(start)).map_err(fail)?;
                return Ok(());
            }

            let end = BytesEnd::new(name);
            writer.write_event(Event::Start(start)).map_err(fail)?;
            if let Some(text) = text {
                writer
                    .write_event(Event::Text(BytesText::new(&cell_to_string(text))))
                    .map_err(fail)?;
            }
            for (child_name, child) in children {
                write_element(writer, child_name, child, opts)?;
            }
            writer.write_event(Event::End(end)).map_err(fail)?;
            Ok(())
        }
        Value::Null => writer
            .write_event(Event::Empty(BytesStart::new(name)))
            .map_err(fail),
        scalar => {
            writer
                .write_event(Event::Start(BytesStart::new(name)))
                .map_err(fail)?;
            writer
                .write_event(Event::Text(BytesText::new(&cell_to_string(scalar))))
                .map_err(fail)?;
            writer
                .write_event(Event::End(BytesEnd::new(name)))
                .map_err(fail)
        }
    }
}

/// Comprobación mínima para no generar XML imposible de volver a leer.
fn is_valid_element_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_alphabetic() || first == '_') {
        return false;
    }
    name.chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn text(bytes: Vec<u8>) -> String {
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn un_elemento_simple_se_reduce_a_su_texto() {
        let value = Xml
            .decode(b"<persona><nombre>Ana</nombre></persona>", &Opts::default())
            .unwrap();
        assert_eq!(value, json!({"persona": {"nombre": "Ana"}}));
    }

    #[test]
    fn los_atributos_llevan_arroba() {
        let value = Xml
            .decode(b"<item id=\"7\" tipo=\"libro\"/>", &Opts::default())
            .unwrap();
        assert_eq!(value, json!({"item": {"@id": 7, "@tipo": "libro"}}));
    }

    #[test]
    fn el_texto_con_atributos_va_en_text() {
        let value = Xml
            .decode(b"<a href=\"x\">pincha</a>", &Opts::default())
            .unwrap();
        assert_eq!(value, json!({"a": {"@href": "x", "#text": "pincha"}}));
    }

    #[test]
    fn los_hijos_repetidos_se_agrupan_en_lista() {
        let value = Xml
            .decode(
                b"<lista><i>1</i><i>2</i><i>3</i></lista>",
                &Opts::default(),
            )
            .unwrap();
        assert_eq!(value, json!({"lista": {"i": [1, 2, 3]}}));
    }

    #[test]
    fn lee_cdata_como_texto() {
        let value = Xml
            .decode(b"<n><![CDATA[a < b]]></n>", &Opts::default())
            .unwrap();
        assert_eq!(value, json!({"n": "a < b"}));
    }

    #[test]
    fn ignora_comentarios_y_declaracion() {
        let value = Xml
            .decode(
                b"<?xml version=\"1.0\"?><!-- nota --><a>1</a>",
                &Opts::default(),
            )
            .unwrap();
        assert_eq!(value, json!({"a": 1}));
    }

    #[test]
    fn ida_y_vuelta_conserva_la_estructura() {
        let opts = Opts::default();
        let original = json!({
            "pedido": {
                "@id": 7,
                "cliente": "Ana",
                "linea": [{"@sku": "A1"}, {"@sku": "B2"}]
            }
        });

        let xml = Xml.encode(&original, &opts).unwrap();
        let recuperado = Xml.decode(&xml, &opts).unwrap();

        assert_eq!(recuperado, original);
    }

    #[test]
    fn escapa_los_caracteres_especiales() {
        let salida = text(
            Xml.encode(&json!({"n": "a < b & c"}), &Opts::default())
                .unwrap(),
        );
        assert!(salida.contains("a &lt; b &amp; c"), "{salida}");

        // Y vuelven sin escapar al leer.
        let value = Xml.decode(salida.as_bytes(), &Opts::default()).unwrap();
        assert_eq!(value, json!({"n": "a < b & c"}));
    }

    #[test]
    fn usa_xml_root_cuando_no_hay_raiz_evidente() {
        let opts = Opts {
            xml_root: "datos".to_string(),
            ..Opts::default()
        };
        let salida = text(Xml.encode(&json!([1, 2]), &opts).unwrap());
        assert!(salida.contains("<datos>"), "{salida}");
    }

    #[test]
    fn un_documento_vacio_da_error_explicado() {
        let error = Xml.decode(b"   ", &Opts::default()).unwrap_err().to_string();
        assert!(error.contains("ningún elemento"), "{error}");
    }

    #[test]
    fn una_etiqueta_sin_cerrar_da_error_explicado() {
        let error = Xml
            .decode(b"<a><b></a>", &Opts::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("XML"), "{error}");
    }

    #[test]
    fn valida_los_nombres_de_elemento() {
        assert!(is_valid_element_name("pedido"));
        assert!(is_valid_element_name("_x-1.a"));
        assert!(!is_valid_element_name("1pedido"));
        assert!(!is_valid_element_name(""));
        assert!(!is_valid_element_name("con espacio"));
    }
}
