# Testing de ds

## Comprobaciones obligatorias

```powershell
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
python scripts/test_tui.py
cargo build --locked --release
```

En PowerShell, comprobar `$LASTEXITCODE` antes de continuar al comando siguiente. No usar `2>&1` con cargo. El workflow ejecuta cada comprobación como un paso independiente.

## Qué protege la suite

- Persistencia: defaults de versiones anteriores, Unicode, notas multilínea, estados y frecuencia, reemplazo del archivo, directorios nuevos, limpieza de temporales, archivos de solo lectura, configuración inválida y conservación de symlinks/permisos Unix.
- Scanner/detección: familias de markers, stacks, managers, scripts, manifests malformados, directorios ignorados, raíces ausentes, deduplicación de raíces idénticas, aplicación de notas/estados y correspondencia de warnings con health.
- Git: estados clean/untracked/staged/deleted, errores frente a clean, repositorios sin commits, commits y HEAD separado, lectura del proceso propio por plataforma, y credenciales de remotes ausentes en el modelo serializado.
- Puertos: paths con quotes, límites de nombres, proyecto anidado más específico, consultas por PID reutilizadas solo dentro de una pasada, validación UTF-16/rangos y 1.024 buffers adversariales deterministas para los parsers de procesos. Esto no sustituye fuzzing continuo ni demuestra ausencia de UB en toda FFI.
- Modelo: compatibilidad JSON sin ports, campos públicos conservados, fechas extremas y counters saturados.
- App/input: navegación vacía y límites, filtros, sorts, selección estable, Unicode en búsqueda, cancelación sin escribir, destinatario capturado para notas/estados/aperturas, fallos de persistencia y recarga posterior de datos guardados.
- UI: todos los modos, listas vacías/con datos, vistas compacta/detallada, tamaños desde 1 × 1 hasta 160 × 45, contenido Unicode largo, estados Git y detalles completos. Las assertions de contenido verifican elementos importantes sin fijar el diseño por píxel.
- CLI: parsing de comandos, argumentos inválidos, resolución exacta/parcial/ambigua, mutations sobre config temporal y scan/list. Los tests de integración invocan el ejecutable real para ayuda/version/errores, verificando códigos de salida, stdout y stderr.

En Windows, la tanda local tiene **128 pruebas normales aprobadas** (125 unitarias/de contratos y 3 de integración), más **cuatro escenarios PTY** aprobados. Los conteos varían por plataforma.

## Terminal real y aislamiento

`scripts/test_tui.py` utiliza ConPTY en Windows y `pty`/termios en Linux/macOS, sin bibliotecas Python externas. Ejecuta el binario de unit tests en un proceso separado y un entry point exclusivo de tests. Cubre q, búsqueda/cancelación, menú/cancelación y Ctrl+C; comprueba salida cero, abandono de pantalla alternativa y cursor restaurado. En Unix también compara la configuración termios anterior/posterior.

`tui::tests::real_terminal_child` figura como ignored en `cargo test` porque necesita esa terminal real y sus fixtures; **el script lo ejecuta explícitamente cuatro veces**. El workflow hace obligatoria la ejecución del script. No es un defecto de producto silenciado.

La configuración se inyecta solo bajo `cfg(test)` mediante un scope thread-local con restauración RAII, probado también durante panic y entre hilos. La app de producción mantiene exactamente su ubicación de configuración. No se cambia PATH, HOME, APPDATA ni el directorio de trabajo global del proceso de pruebas. Las carpetas temporales y subprocesses se limpian al terminar.

Las pruebas de CLI que escriben ejecutan sus handlers reales dentro de este scope; no son invocaciones externas de comandos mutadores. En Windows no se puede aislar la configuración del ejecutable de producción simplemente alterando APPDATA, por las APIs KnownFolder. Ninguna prueba mutadora debe asumirlo.

## Cobertura medible

Herramientas de desarrollo, no dependencias de Cargo.toml:

```powershell
cargo install cargo-llvm-cov --version 0.9.1 --locked
rustup component add llvm-tools-preview
python scripts/coverage.py --minimum-lines 75
```

El script recoge unit tests, integración CLI y sesiones PTY instrumentadas en un único informe. Genera `target/coverage/summary.json`, `lcov.info` y HTML. El directorio target está ignorado y los informes de CI quedan como artefactos descargables.

El gate inicial es **75 % de líneas**. Se excluyen los archivos de pruebas independientes del informe; las pruebas inline dentro de módulos Rust siguen en las estadísticas de LLVM. Por ello el porcentaje representa los archivos instrumentados reportados, no una medida exacta aislada de producción. No se presume cobertura de ramas, cobertura de código C/libgit2 ni ausencia de defectos. El informe por archivo identifica dónde falta trabajo, especialmente lanzamientos externos, errores de terminal y autodiscovery del entorno.

La medición preliminar local fue 77,7 % antes de la última ampliación de casos Git/UI; el informe generado es la referencia para el porcentaje de cada commit.

## Automatización de GitHub

`.github/workflows/test.yml` corre en push a main, pull requests y ejecución manual. Matriz de Windows/Linux/macOS: formato, Clippy, unit/integration, terminal real y build release. Windows además recoge cobertura, impone el gate y publica los informes. Las acciones checkout y upload-artifact están fijadas a commits; se usa Cargo.lock con --locked y el toolchain stable.

Agregar un workflow no configura una regla de protección de rama. Si se exige impedir merges que fallen, se deben marcar estos jobs como required checks en las reglas del repositorio. No se cambiaron reglas administrativas del repositorio.

## Límites conocidos que deben seguir visibles

Esta tanda conserva el comportamiento del producto. No transforma en assertions de comportamiento correcto los defects aún pendientes: identidad relativa/aliases, conflictos entre instancias, opciones del scanner sin aplicar, profundidad/monorepos, actividad de archivos fuente, descubrimiento de targets Cargo, preferencias/scroll de UI, comandos inferidos y launcher Linux. Están descritos en AUDITORIA_RUST.md y deben acompañar sus futuras correcciones con regresiones.

No se ejecutan editores externos, no se abren carpetas personales y no se depende de red en los tests. No hay todavía simulación de power loss real, pruebas de carga sostenida con miles de proyectos, Miri de toda la FFI ni fuzzing continuo. Esas pruebas requieren harnesses y criterios específicos; contar más tests no las sustituye.
