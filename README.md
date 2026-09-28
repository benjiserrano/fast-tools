# Fast tools

Navaja suiza portable para desarrolladores: conversión de formatos y utilidades
del día a día en un solo `.exe`, sin instalación y **sin enviar nada a internet**.

Se copia a una carpeta o a un USB y se ejecuta. Los datos de la aplicación se
guardan en `fast-tools-data/`, junto al ejecutable. No escribe en el registro de
Windows.

## Estado

Las seis fases del plan están implementadas: **39 herramientas** funcionando.

| Fase | Contenido | Estado |
| ---- | --------- | ------ |
| 0 | Andamiaje, portabilidad, catálogo, shell de UI | hecho |
| 1 | Conversión de datos: CSV, JSON, YAML, TOML, XML, XLSX | hecho |
| 2 | Texto y datos · Cripto e identificadores | hecho |
| 3 | Imágenes, QR, EXIF, favicon, color | hecho |
| 4 | Motores externos bajo demanda: documentos, audio y vídeo | hecho |
| 5 | Web y red: regex, cron, HTTP, URL, fechas | hecho |

Pendientes recogidos en `BACKLOG.md`.

## Requisitos de desarrollo

- Rust estable con el toolchain `x86_64-pc-windows-msvc`
- Node 20 o superior y pnpm
- Microsoft Edge WebView2 Runtime (viene de serie en Windows 11)

## Comandos

```bash
pnpm install          # dependencias de la interfaz
pnpm dev              # ventana de desarrollo con recarga en caliente
pnpm build            # .exe portable en src-tauri/target/release/

pnpm typecheck        # tsc sobre la app y sobre los tests por separado
pnpm test             # tests de la interfaz (Vitest)
cargo test --manifest-path src-tauri/Cargo.toml   # tests del núcleo

# Tests que usan la red; no entran en la suite normal.
cargo test --manifest-path src-tauri/Cargo.toml -- --ignored
```

El código de la aplicación y el de los tests se comprueban con dos `tsconfig`
distintos a propósito: la app corre en un WebView y no debe ver los globales de
Node, mientras que los tests sí los necesitan para leer `registry.rs` del disco.

## Arquitectura

```
src/          Interfaz (React + Vite + Tailwind), ejecuta en WebView2
src-tauri/    Núcleo Rust: catálogo, conversores, criptografía, imágenes
```

Reparto de trabajo: las transformaciones puras de string que responden al
instante se ejecutan en el WebView; todo lo que toque disco, criptografía,
imágenes o red se ejecuta en Rust. El campo `runtime` del catálogo indica cuál
de los dos lleva cada herramienta.

El catálogo de `src-tauri/src/registry.rs` es la fuente única de verdad: la
barra lateral y la paleta `Ctrl+K` se construyen a partir de él. Para añadir una
herramienta se añade su fila allí y su panel en `src/tools/`, registrado en
`src/tools/panels.ts`.

### Almacenamiento

`src-tauri/src/portable.rs` es el único punto del programa que decide dónde se
escribe. Prueba por orden:

1. `<directorio del exe>/fast-tools-data` — modo portable
2. `%LOCALAPPDATA%/fast-tools` — si el anterior es de solo lectura
3. `%TEMP%/fast-tools` — último recurso

La barra de estado muestra el modo activo y, si no es portable, el motivo.

La ventana se crea desde `lib.rs` y no desde `tauri.conf.json` por este motivo:
hay que redirigir el directorio de datos de WebView2 (caché, cookies,
localStorage) a `fast-tools-data/webview2`. Por defecto Tauri lo sitúa en
`%LOCALAPPDATA%`, lo que dejaría rastro de la sesión en la máquina en vez de
viajar con el ejecutable.

## Seguridad

- Todo el procesamiento es local. La app no hace peticiones de red salvo las que
  el usuario inicia de forma explícita.
- Los motores externos (FFmpeg, Pandoc, LibreOffice) de la Fase 4 se descargan
  bajo demanda, con consentimiento previo, desde el dominio oficial del proyecto
  y con el hash SHA-256 fijado en el código. Un hash que no coincide aborta la
  operación y borra la descarga.
- Los procesos hijo se lanzan sin shell, con los argumentos como vector.

El `.exe` no está firmado, por lo que SmartScreen mostrará un aviso la primera
vez. Los hashes SHA-256 de cada versión se publican con la release.
