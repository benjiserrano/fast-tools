# Pendiente

Cosas acordadas que no entran en la fase en curso. Las fases del plan van en
`README.md`; esto es lo que se ha ido apuntando por el camino.

## 1. Codificación de los archivos que se leen

**Es un fallo, no una mejora.** Hoy `as_text`
(`src-tauri/src/convert/codec/mod.rs`) y `load_text_file`
(`src-tauri/src/commands/convert.rs`) rechazan en seco cualquier archivo que no
sea UTF-8, con el mensaje «El contenido no es texto UTF-8 válido». Un CSV
exportado desde Excel en español viene en Windows-1252 y ahora mismo no se
puede ni abrir.

Qué hace falta:

- Detectar la codificación (`chardetng`) y transcodificar a UTF-8
  (`encoding_rs`) al leer.
- Reconocer la marca de orden de bytes de UTF-16 LE y BE, no solo la de UTF-8.
- Mostrar en la interfaz qué codificación se ha detectado y permitir forzar
  otra a mano, porque la detección heurística falla en archivos cortos.
- Escribir siempre UTF-8, sin marca de orden.

Cobertura mínima: UTF-8 con y sin BOM, UTF-16 LE/BE, Windows-1252, ISO-8859-1.

## 2. Bandeja del sistema y buscador superpuesto

Cerrar la ventana debe minimizar a la bandeja en vez de salir del programa. Con
la aplicación en la bandeja, el atajo global `Ctrl+Alt+T` abre una ventana
superpuesta —sin marco, siempre encima, fuera de la barra de tareas— con el
mismo buscador de herramientas de la paleta. Al elegir una herramienta, se abre
la ventana principal ya situada en ella.

Qué hace falta:

- `tauri-plugin-global-shortcut` y `TrayIcon`.
- Una segunda ventana con `decorations: false`, `alwaysOnTop: true`,
  `skipTaskbar: true`, creada al arrancar y oculta hasta que se invoque.
- Menú de bandeja con «Abrir», «Buscar herramienta» y «Salir»; salir de verdad
  solo desde ahí.
- **Contemplar que el atajo esté ocupado por otra aplicación**: el registro
  falla y hay que avisarlo en la interfaz, no quedarse callado. Conviene dejar
  el atajo configurable.
