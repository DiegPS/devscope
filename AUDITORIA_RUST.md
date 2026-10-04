# Auditoría de ds / devscope

## Segunda tanda: pendientes internos resueltos

Se retomaron los ajustes internos de los hallazgos 16, 22 y 24, manteniendo el diseño, los comandos, el formato JSON/TOML y las dependencias:

- La detección de puertos consulta cada PID una sola vez por pasada y reutiliza su asociación de proyecto, incluidos los casos sin resultado. El caché termina con esa pasada; las siguientes vuelven a consultar. Se conservan la selección del proyecto más específico, la deduplicación de puertos y el orden ascendente.
- Los ciclos de filtros y orden usan slices estáticos en lugar de construir un Vec en cada pulsación. Se eliminó la llamada al método vacío de prioridad de selección y su estado auxiliar, que no realizaban ninguna acción.
- Las pruebas de descubrimiento usan candidatos temporales explícitos y ya no recorren los proyectos personales del usuario. La producción conserva sus mismos candidatos, profundidad, orden y reglas de recorrido.
- La prueba de niveles de salud comprueba la función utilizada en producción y sus límites; se eliminó el algoritmo duplicado dentro del test.
- Salud evita overflow con fechas extremas y suma de contadores de Git, sin cambiar sus criterios ni penalizaciones normales.

Validación de esta tanda en Windows: formato, cargo check --locked, cargo clippy --locked --all-targets -- -D warnings, cargo test --locked (**76/76**), cargo build --locked y git diff --check pasan. Se ejecutó la TUI recién compilada en ConPTY y se cerró con q: salida cero, restauración de pantalla alternativa y hash de configuración sin cambios. No se reinstaló esta segunda tanda ni se hizo commit/push.

Siguen pendientes la migración de identidad de rutas (01), los conflictos entre guardados de distintas instancias (parte de 03), las decisiones de scanner/discovery (07–09), actividad (12), targets de artefactos (13), opciones visuales (14), comandos y detecciones inferidas (17, 19 y parte de 23), coordinación del pipeline (21), parte de las invariantes del modelo (22), y CI/MSRV (parte de 24). Reutilizar un único vector de warnings requiere conservar el contrato JSON actual, que incluye warnings tanto en Project como en health; no se eliminó ese campo público.

El análisis de velocidad quedó documentado por separado en RENDIMIENTO.md. Los ajustes nuevos no afirman ganancias de latencia sin una comparación posterior.

## Correcciones internas aplicadas después de la auditoría

Se aplicaron las reparaciones autorizadas que conservan la apariencia, los comandos, el esquema de configuración y las dependencias existentes. La copia local se actualizó por fast-forward al cambio de nombre que ya estaba en GitHub. No se hicieron commits ni push de estas reparaciones.

- Selección por identidad durante reordenamientos y destinatario capturado para notas/estados; actualización de filtros después de guardar.
- Guardado por archivo temporal sincronizado y reemplazo en el mismo directorio, conservando symlinks existentes. Los fallos mantienen el borrador y no actualizan el estado en memoria como si hubiera persistido. Esto no resuelve todavía conflictos entre instancias ni constituye una garantía universal ante fallos del dispositivo o filesystem.
- Git reconoce archivos nuevos staged y conflictos; propaga errores de consulta.
- Guard de terminal para restauración al retornar errores o durante unwinding; procesos externos con stdio aislado cuando no son interactivos, espera de hijos y contabilización de aperturas sin duplicados.
- Validación del buffer de cmdline de Windows y cierre RAII de handles; parser de macOS corregido, límites de asociación de paths y elección de proyecto más específico.
- Tests de resolución de comandos sin modificar PATH global; protección contra overflow en scores y eliminación de clones innecesarios.
- Defaults nuevos de VS Code/Cursor usan la ruta seleccionada. Se conservan las acciones personalizadas y las configuraciones ya guardadas.
- Cargo.lock queda disponible para versionar, sin añadir ni cambiar dependencias en Cargo.toml.

Se dejaron pendientes las decisiones que cambiarían reglas de producto o requerirían una migración: identidad canónica de rutas y datos antiguos, política de profundidad/ignores/monorepos, cálculo de actividad/health, detección ampliada de targets, nuevas preferencias visuales y reorganización del pipeline de scan. El resto del documento conserva el diagnóstico histórico y sus referencias originales.

Validación final de las reparaciones en Windows: cargo fmt --check, cargo check --locked, cargo clippy --locked --all-targets -- -D warnings, cargo test --locked (73/73) y cargo build --locked pasan. Se probaron lectura real del cmdline del propio proceso, reemplazo de config, cleanup en fallos, edición con cambio de selección, borradores ante errores y actualización de filtros. La CLI compilada pasó --version y --help. No se ejecutó la TUI interactiva ni las ramas Unix de sistema; el parser macOS sí tiene prueba portable. No se tocó la configuración personal: se rechazó un smoke con escrituras al comprobar que las APIs Windows de carpetas conocidas no redirigen config por cambiar APPDATA/LOCALAPPDATA.

Fecha: 4 de octubre de 2026. Revisión de los 23 archivos Rust, Cargo.toml, instrucciones del repositorio y documentación relevante.

La base es aprovechable: módulos con responsabilidades reconocibles, enums para los modos de la TUI, separación de detección y presentación, uso razonable de Rayon, canales para entregar resultados y configuración compatible mediante defaults de Serde. Los problemas principales son de identidad, persistencia, exactitud de los datos y coordinación de la interfaz. No hace falta rehacer toda la app ni introducir un runtime async para corregirlos.

## Alcance y evidencia

- Checkout local: `1f5e877`, sin modificaciones previas. GitHub: `e3a34a9`, un commit por delante. Se leyó el diff completo; contiene el cambio de nombre visible a ds, formato, ajustes de compilación condicional y una prueba de CLI. Los problemas funcionales siguientes siguen presentes en GitHub.
- No se hizo pull, commit, push ni modificación del código de producción. Este informe es el único archivo añadido al repositorio. Cargo generó un Cargo.lock que el repositorio ignora y actualizó los artefactos de compilación.
- Windows fue la plataforma ejecutada. Linux y macOS se revisaron por código, sin ejecución. No se realizó una sesión interactiva real de terminal; se usó TestBackend para comprobar comportamiento del renderizado. No se realizó un análisis de vulnerabilidades de todas las dependencias.
- Las pruebas diagnósticas se ejecutaron en copias temporales. Sus assertions verifican el comportamiento defectuoso actual; que pasen confirma la reproducción, no que la app esté corregida.

| Comprobación | Checkout local | Snapshot de GitHub |
|---|---|---|
| cargo check | Pasa | Cubierto por Clippy con todos los targets |
| cargo build --locked | Pasa | No ejecutado por separado |
| cargo clippy -- -D warnings | Pasa | Pasa con todos los targets |
| cargo clippy --all-targets -- -D warnings | Falla: items después del módulo de tests en config.rs | Pasa; ese cambio ya está en GitHub |
| cargo test, ejecución paralela normal | 57 pasan, 1 falla por interferencia sobre PATH | No ejecutado en paralelo |
| cargo test -- --test-threads=1 | 58 pasan | 59 pasan |
| cargo fmt -- --check | Falla por formato | Pasa; ya arreglado en GitHub |
| Diagnósticos adicionales | 10 casos iniciales reproducidos | 15 casos reproducidos |

Las fuentes de los diagnósticos se conservaron fuera del repositorio:

- [13 diagnósticos generales](C:/Users/dpahu/.codex/visualizations/2026/10/04/01a10608-4920-79c3-a1a6-d44d4f78570f/ds-audit/audit_regressions.rs).
- [2 diagnósticos de puertos](C:/Users/dpahu/.codex/visualizations/2026/10/04/01a10608-4920-79c3-a1a6-d44d4f78570f/ds-audit/audit_ports.rs).

## Hallazgos prioritarios

P1 significa que conviene arreglarlo primero por el riesgo para datos o seguridad de memoria. P2 identifica errores de comportamiento y fiabilidad. P3 identifica mantenimiento o endurecimiento. Se distingue reproducción de inspección de código.

### 01 — P1: un proyecto no tiene una identidad estable

[config.rs:199](C:/Users/dpahu/devscope/src/config.rs:199), [scanner.rs:151](C:/Users/dpahu/devscope/src/scanner.rs:151), [scanner.rs:343](C:/Users/dpahu/devscope/src/scanner.rs:343).

`normalize_path` solo simplifica componentes; no convierte una ruta relativa en absoluta ni resuelve aliases. `dedupe_projects` compara PathBuf literalmente. La misma carpeta configurada por ruta absoluta y por `.` produce dos proyectos: confirmado con un diagnóstico. El proyecto raíz escaneado mediante `.` tiene id `.`. Esto también puede guardar notas, estados y frecuencia bajo una clave compartida por diferentes carpetas desde las que ejecutes ds.

Corrección: centralizar la construcción de un ProjectId a partir de una ruta absoluta estable. Para carpetas existentes, decidir explícitamente si se unifican symlinks mediante canonicalize. Aplicar la misma política al scanner, CLI, sesiones temporales, notas, estados y scores. Migrar claves existentes con cuidado; no basta con cambiar únicamente el deduplicado. En Windows, revisar aliases, casing y rutas UNC. Preservar PathBuf/OsString en operaciones de filesystem y reservar la conversión a texto para presentación/persistencia con una política definida.

### 02 — P1: una nota o estado puede guardarse en otro proyecto

[app.rs:267](C:/Users/dpahu/devscope/src/app.rs:267), [app.rs:527](C:/Users/dpahu/devscope/src/app.rs:527), [input.rs:125](C:/Users/dpahu/devscope/src/input.rs:125), [input.rs:169](C:/Users/dpahu/devscope/src/input.rs:169).

La selección es una posición dentro de `filtered_indices`. Cuando la hidratación de Git cambia DirtyFirst o el filtro Dirty, se reordena la lista pero se conserva la posición. La selección puede pasar de A a B durante EditingNote o ChangingStatus. Enter vuelve a resolver el proyecto seleccionado en ese momento. El diagnóstico reproduce el cambio de identidad durante la edición; la escritura al proyecto equivocado se deduce directamente del handler, sin realizar escrituras en la configuración personal.

Corrección: preservar la selección por ProjectId al regenerar la vista y capturar el id editado al entrar al modo. Por ejemplo, representar la edición como estado con datos: `EditingNote { project_id, draft }`. La acción pendiente debe mantener su propio destinatario aunque la vista cambie. Si desaparece el proyecto, cancelar con un mensaje claro.

### 03 — P1: el guardado puede perder datos y ocultar el fallo

[config.rs:320](C:/Users/dpahu/devscope/src/config.rs:320), [input.rs:133](C:/Users/dpahu/devscope/src/input.rs:133), [input.rs:173](C:/Users/dpahu/devscope/src/input.rs:173), [tui.rs:278](C:/Users/dpahu/devscope/src/tui.rs:278).

`std::fs::write` reemplaza el config directamente. Una escritura interrumpida puede dejar un archivo parcial. Muchos llamadores ignoran el Result y actualizan la memoria como si hubiera persistido. Además, una TUI con un Config antiguo puede sobrescribir cambios hechos por otra instancia, por el CLI o por el editor externo que la propia app permite abrir. Son riesgos comprobables por el flujo del código; no se provocó corrupción del archivo real.

Corrección: serializar primero, escribir a un temporal en el mismo directorio y reemplazar con una estrategia compatible con Windows. Propagar fallos y mostrar estado de persistencia. Para varias instancias, usar bloqueo y recargar/combinar las mutaciones durante la transacción, o detectar conflictos y pedir resolución. Un rename atómico por sí solo no resuelve la pérdida de actualizaciones entre snapshots antiguos.

### 04 — P1: el unsafe de Windows carece de garantías de memoria explícitas

[ports.rs:136](C:/Users/dpahu/devscope/src/ports.rs:136), [ports.rs:151](C:/Users/dpahu/devscope/src/ports.rs:151), [ports.rs:157](C:/Users/dpahu/devscope/src/ports.rs:157).

Se convierte la dirección de un Vec<u8> en referencia a UnicodeString. Vec<u8> no promete la alineación de esa estructura. Después se crea un slice de u16 desde el puntero devuelto sin verificar alineación, longitud par ni pertenencia del rango a la asignación recibida. Limitar a 4096 elementos no demuestra esas condiciones. No reproduje un crash ni comportamiento indefinido; el hallazgo es que el contrato de seguridad de estas operaciones no está establecido en el código.

Corrección: encapsular la FFI, usar un handle con Drop y bindings tipados cuando sea viable; leer la cabecera con alineación garantizada o read_unaligned; validar el rango y decodificar bytes a u16 sin construir referencias inválidas. Añadir comentarios SAFETY que justifiquen cada operación, tamaños y lifetimes. Las condiciones están documentadas en [from_raw_parts](https://doc.rust-lang.org/std/slice/fn.from_raw_parts.html) y [read_unaligned](https://doc.rust-lang.org/std/ptr/fn.read_unaligned.html).

### 05 — P2: Git marca como limpio un archivo nuevo staged

[git.rs:143](C:/Users/dpahu/devscope/src/git.rs:143).

Se cuentan modificaciones, borrados, renames y typechanges, pero no INDEX_NEW ni conflictos. Confirmado: crear un archivo, hacer git add y dejarlo pendiente produce DirtyStatus::Clean en la app. El filtro Dirty y health reciben información incorrecta.

Corrección: definir cómo clasificar cada flag de Git; considerar staged additions y conflictos como cambios. Usar pruebas de repositorios temporales para staged-only, conflictos, renames, borrados y archivos nuevos. Los recuentos deben contar paths con una política explícita, no tratar cada flag como un archivo distinto.

### 06 — P2: un error de Git se convierte en clean

[git.rs:151](C:/Users/dpahu/devscope/src/git.rs:151).

Si repo.statuses falla, `get_working_tree_status` devuelve (0, 0), y `get_git_status` lo convierte en Clean. El manejo externo de DirtyStatus::Error no alcanza este error porque fue descartado antes.

Corrección: devolver Result desde la función interna y conservar el fallo hasta la presentación. Unknown, Checking, Clean y Error deben permanecer distinguibles también en la columna de Git, que actualmente usa estilo clean para cualquier estado que no sea Dirty.

### 07 — P2: tres opciones del scanner no se aplican

[config.rs:21](C:/Users/dpahu/devscope/src/config.rs:21), [scanner.rs:205](C:/Users/dpahu/devscope/src/scanner.rs:205).

`respect_gitignore`, `scan_hidden` y `follow_symlinks` se deserializan, pero collect_subdirs solo recibe max_depth. El walker fuerza git_ignore(false) y follow_links(false), y el callback descarta carpetas ocultas. Se reprodujo la inclusión de una carpeta gitignored y la exclusión de una carpeta oculta con scan_hidden=true. El caso symlink se comprobó por código.

Corrección: pasar una política de scan al walker y compartirla con discovery. Si se habilitan links, revisar ciclos y deduplicación por identidad. La lista SKIP_DIRS y los ignores deben tener una precedencia documentada.

### 08 — P2: el monorepo cambia según la raíz configurada

[scanner.rs:195](C:/Users/dpahu/devscope/src/scanner.rs:195), [scanner.rs:266](C:/Users/dpahu/devscope/src/scanner.rs:266).

Si una subcarpeta es proyecto, el callback devuelve false y deja de descender. Si esa misma carpeta se configura como raíz, se analiza y después sí se recorren sus hijos. Confirmado: `outer/inner` devuelve un proyecto al escanear el contenedor, y dos al escanear outer. Esto contradice la intención declarada de detectar estructuras de monorepo.

Corrección: separar detectar un proyecto de podar su árbol. Definir si un workspace y sus paquetes se muestran juntos, como jerarquía o como proyectos independientes; aplicar esa regla siempre.

### 09 — P2: profundidad inconsistente entre scanner y discovery

[scanner.rs:213](C:/Users/dpahu/devscope/src/scanner.rs:213), [discover.rs:205](C:/Users/dpahu/devscope/src/discover.rs:205).

El scanner usa max_depth + 1 y discovery usa max_depth. Confirmado: con max_depth=1 el scanner detecta un proyecto a dos niveles. Si existe una razón para contar también el nivel de markers, debe explicitarse; actualmente los dos recorridos interpretan la opción de forma diferente.

Corrección: fijar la definición del nivel raíz, compartirla y verificar boundaries 0, 1 y 4.

### 10 — P2: VS Code y Cursor abren la carpeta incorrecta

[config.rs:401](C:/Users/dpahu/devscope/src/config.rs:401), [config.rs:411](C:/Users/dpahu/devscope/src/config.rs:411), [tui.rs:184](C:/Users/dpahu/devscope/src/tui.rs:184).

Ambas acciones tienen args=["."] y current_dir=false. Se confirmó que la ruta del proyecto no entra en los argumentos. Por eso el proceso recibe el directorio desde el que se lanzó ds.

Corrección: pasar `{path}` explícitamente o configurar el cwd del hijo al proyecto. Probar argumentos y cwd con un ejecutable auxiliar que registre la invocación; no hace falta lanzar un editor real para validarlo.

### 11 — P2: el terminal no tiene restauración robusta

[tui.rs:20](C:/Users/dpahu/devscope/src/tui.rs:20), [tui.rs:207](C:/Users/dpahu/devscope/src/tui.rs:207).

Después de enable_raw_mode, fallos en EnterAlternateScreen o Terminal::new retornan sin restaurar. Un panic tampoco ejecuta la limpieza normal. Incluso en el final normal, un error de LeaveAlternateScreen o show_cursor puede evitar disable_raw_mode. suspend_and_run ignora errores de transición y del hijo.

Corrección: un guard de terminal con Drop y limpieza best effort para todos los recursos adquiridos, más un panic hook apropiado. Hacer suspend/resume explícitos y devolver el resultado del proceso. App::new y el escaneo inicial deberían ejecutarse antes de entrar en raw mode, o mostrar una pantalla de carga controlada.

### 12 — P2: editar contenido en src no necesariamente actualiza actividad

[scanner.rs:432](C:/Users/dpahu/devscope/src/scanner.rs:432).

Se lee el mtime del directorio src, no el de sus archivos. Cambiar contenido de un archivo existente no implica cambiar el mtime de su padre. Confirmado con un archivo fuente reciente dentro de un src antiguo: el proyecto conserva actividad de más de 90 días. Esto afecta sort, estado inferido y health.

Corrección: definir una fuente de actividad que incluya archivos relevantes con recorrido acotado y skips, o distinguir claramente actividad de manifests, trabajo local y commits. Medir el costo y cachear si es necesario. La fecha del checkout tampoco debe confundirse con actividad original de desarrollo.

### 13 — P2: los artefactos Rust se adivinan con el nombre de la carpeta

[artifacts.rs:133](C:/Users/dpahu/devscope/src/artifacts.rs:133).

Se construye target/debug/<folder>.exe. No se leen package.name ni [[bin]], y tampoco se contempla un target-dir distinto o targets de workspace. Confirmado con un [[bin]] llamado ds y un ds.exe existente: la app lo informa como ausente. El propio rename de devscope a ds expone este problema.

Corrección: usar metadata de Cargo o parsear los targets con defaults correctos; evitar invocar Cargo para cada proyecto en cada frame o scan. Distinguir bibliotecas de aplicaciones y elegir entre debug/release con una política explícita.

### 14 — P2: preferencias UI y scroll de ayuda sin efecto

[ui/mod.rs:15](C:/Users/dpahu/devscope/src/ui/mod.rs:15), [ui/mod.rs:191](C:/Users/dpahu/devscope/src/ui/mod.rs:191).

theme, show_icons y right_panel existen en Config pero el render utiliza Theme::default y decide el panel solo con ViewMode. help_scroll se incrementa en input pero el Paragraph no recibe scroll. Dos diagnósticos con TestBackend confirmaron buffers idénticos al cambiar estas opciones. En ventanas normales, parte de la ayuda y detalles largos quedan fuera sin una forma efectiva de leerlos.

Corrección: conectar las opciones o eliminarlas del contrato visible, aplicar scroll y establecer límites de navegación. Probar ventanas pequeñas y distintos tamaños con TestBackend; el render no sustituye una prueba manual real del terminal.

### 15 — P2: filtros y orden quedan obsoletos tras editar

[input.rs:125](C:/Users/dpahu/devscope/src/input.rs:125), [input.rs:169](C:/Users/dpahu/devscope/src/input.rs:169).

Guardar una nota o estado cambia el proyecto pero no reaplica filter/sort. Un proyecto puede seguir visible en Active después de pasar a Archived, o en WithNotes después de borrar su nota. RecordVisit y RecordOpen tampoco refrescan de inmediato un orden basado en Score.

Corrección: recalcular las vistas después de cada mutación relevante, preservando identidad de selección como en el hallazgo 02. Modelar una acción de dominio que actualice estado y vista de forma consistente.

### 16 — P2: la asociación de puertos falla con quotes y proyectos anidados

[ports.rs:39](C:/Users/dpahu/devscope/src/ports.rs:39), [ports.rs:57](C:/Users/dpahu/devscope/src/ports.rs:57).

El matcher no acepta la comilla de cierre después del path y elige el primer proyecto que hace match. Se reprodujeron ambos casos: un path quoted exacto no coincide; un proceso de un subproyecto se asigna al padre si aparece primero. Solo mirar cmdline tampoco identifica un proceso lanzado como `node server.js` desde un cwd de proyecto.

Corrección: usar argumentos/cwd de proceso cuando la plataforma los permita; validar límites antes y después del path, y elegir el proyecto más específico. Cachear cmdline por pid durante un scan, ya que un proceso puede tener varios listeners. Conservar una distinción entre no encontrado y permiso denegado.

### 17 — P2: un package.json sin scripts pierde hasta install

[commands.rs:53](C:/Users/dpahu/devscope/src/commands.rs:53), [commands.rs:100](C:/Users/dpahu/devscope/src/commands.rs:100).

parse_package_json_scripts devuelve None si no hay scripts reconocidos; detect_node_commands retorna antes de añadir install. Confirmado con `{}`: no se sugiere ningún comando y se penaliza health, pese a que install es válido.

Corrección: distinguir JSON inválido de scripts vacíos y detectar install de forma independiente. Limitar a seis comandos al presentar, en vez de truncar los datos y eliminar comandos de otros stacks por orden de detección.

### 18 — P2: spawn puede interferir con la TUI y no se gestiona el hijo

[tui.rs:184](C:/Users/dpahu/devscope/src/tui.rs:184), [tui.rs:237](C:/Users/dpahu/devscope/src/tui.rs:237).

Las acciones sin terminal_mode usan spawn con stdout/stderr heredados. Un launcher que escribe mensajes puede dibujar encima del alternate screen. Se descarta Child; en Unix no esperar hijos terminados puede dejar zombies mientras ds siga ejecutándose. Las acciones terminal_mode cuentan opens dos veces, y varios caminos cuentan aperturas fallidas como exitosas.

Corrección: definir stdio por tipo de acción, gestionar wait/reaping y reportar exit status/spawn errors. Registrar una sola apertura cuando exista éxito según una regla definida. Para editores desacoplados, resolver explícitamente el ciclo de vida del launcher.

### 19 — P2: configuración de terminal equivocada para Linux

[config.rs:475](C:/Users/dpahu/devscope/src/config.rs:475).

La rama cfg(not(windows)) configura `open -a Terminal {path}`, que corresponde a macOS; también se aplica en Linux. Hallazgo por inspección, sin ejecución Linux.

Corrección: separar Windows, macOS y Linux. En Linux, usar una preferencia configurable y detectar un launcher disponible; no asumir que una única aplicación de terminal existe en todas las distribuciones.

### 20 — P2: el parser de argumentos de macOS omite el último argumento

[ports.rs:229](C:/Users/dpahu/devscope/src/ports.rs:229).

Tras saltar exec_path y ceros de padding, pos apunta a argv[0]. Se añade exec_path a args y luego se leen argc-1 entradas empezando en argv[0]. Esto duplica habitualmente el executable/argv[0] y deja sin leer el último argumento, que puede contener la ruta del proyecto. Hallazgo por inspección, sin ejecución macOS.

Corrección: separar el parser puro del sysctl y probar buffers con argc 1, 2 y varios, padding, argumentos quoted y datos truncados. Respetar el tamaño efectivo devuelto por sysctl.

## Diseño, rendimiento y mantenimiento

### 21 — P2: demasiado trabajo de filesystem en el hilo de UI

[app.rs:225](C:/Users/dpahu/devscope/src/app.rs:225), [app.rs:513](C:/Users/dpahu/devscope/src/app.rs:513), [tui.rs:41](C:/Users/dpahu/devscope/src/tui.rs:41).

reload hace el scan sincrónicamente. Cada resultado Git provoca un DirSnapshot nuevo para recomputar health dentro del event loop. El loop renderiza cada 50 ms aunque no cambie nada, y los resultados de Git pueden provocar reordenaciones frecuentes. Las recargas lanzan nuevos workers y descartan receivers anteriores, pero no cancelan el trabajo anterior. La generación protege resultados antiguos de Git, lo cual está bien; no limita el trabajo desperdiciado.

Mejora: un worker coordinado para scan y enriquecimiento que devuelva datos completos, snapshots reutilizables y resultados agrupados; cancelación cooperativa o coalescing de reloads; redraw solo cuando hay eventos, cambios o un tick necesario. Medir antes/después con pocos proyectos, cientos, un monorepo y un disco lento. No se midió una ganancia de rendimiento ni se afirma que Rayon sea el cuello de botella.

### 22 — P3: el modelo permite estados contradictorios

[project.rs:235](C:/Users/dpahu/devscope/src/project.rs:235), [project.rs:244](C:/Users/dpahu/devscope/src/project.rs:244), [config.rs:37](C:/Users/dpahu/devscope/src/config.rs:37).

Project.warnings duplica Project.health.warnings; id duplica una representación textual del path; Config.project_status almacena String aunque existe ProjectStatus. Hay campos derivados públicos mutables y enums/variantes sin flujo implementado, como prioridad de hidratación: prioritize_selected está vacío. Esto obliga a recordar manualmente invariantes entre módulos.

Mejora Rust: un ProjectId explícito, un único lugar para warnings, tipos para estados/tecnologías/managers y visibilidad mínima. Usar Option<&GitInfo> donde se consulta un dato opcional, en vez de &Option<GitInfo>. Copy para enums pequeños cuando proceda, slices/static arrays para los ciclos de filtros, y getters o acciones para mutaciones sensibles. Un lib.rs facilitaría pruebas de integración; main.rs puede concentrarse en CLI y arranque.

No es necesario reemplazar todos los Strings: notas, branch names y etiquetas libres deben seguir siendo texto. Tampoco hay que quitar unwrap indiscriminadamente: los dos unwrap de latest están protegidos por short-circuit y los unwrap de tests son apropiados.

### 23 — P3: detección y health deben presentarse como heurísticas

[detect.rs:9](C:/Users/dpahu/devscope/src/detect.rs:9), [health.rs:23](C:/Users/dpahu/devscope/src/health.rs:23), [discover.rs:265](C:/Users/dpahu/devscope/src/discover.rs:265).

pubspec.yaml se identifica siempre como Flutter/Dart y se sugieren comandos flutter también para un paquete Dart puro. Java recibe comandos Spring Boot sin comprobar el plugin. Búsquedas de substring en manifests pueden confundir comentarios, nombres de scripts o metadatos con dependencias. La detección .NET se desactiva si hay Cargo.toml, ocultando un stack mixto. Bun se interpreta de forma diferente en stack, manager y comandos. Composer es marker sin un stack PHP correspondiente.

Health resta puntos por cualquier .env.*, incluso .env.example, y penaliza una feature branch aunque sea trabajo normal. Estos criterios son decisiones de producto, no una medición universal de calidad o de exposición de secretos. Tener un .env local ignorado no prueba que se publicó una credencial. No se leen los contenidos de env, lo cual es una buena restricción existente.

Mejora: parsear JSON/TOML por estructura, compartir la detección de manager y declarar confianza de comandos inferidos. Hacer los criterios de health explícitos/configurables y separar hechos de consejos. En discovery, el desempate de confidence es ascendente: Low se ordena antes de High con igual count; invertirlo si la intención es priorizar confianza alta.

### 24 — P3: la suite y la distribución no aseguran reproducibilidad

[tui.rs:290](C:/Users/dpahu/devscope/src/tui.rs:290), [.gitignore:3](C:/Users/dpahu/devscope/.gitignore:3), [Cargo.toml:1](C:/Users/dpahu/devscope/Cargo.toml:1).

Las pruebas de resolve_command modifican PATH globalmente en paralelo. Se observó una falla real en ejecución normal y éxito de las 58 al ejecutarlas en serie. Un mutex usado solo por esas pruebas reduce interferencia interna, pero la solución más aislada es pasar la lista de paths/extensions a un resolver puro y no modificar el entorno global. Si PATH no existía al inicio, algunos tests tampoco lo restauran a ausencia; un panic evita su restauración.

Hay tests que repiten la implementación, como un helper local para health_level_mapping, y tests de discovery que recorren directorios reales del usuario. Los principales flujos de input, persistencia y errores de scan no estaban cubiertos. El AGENTS.md local todavía decía 42 tests; actualmente son 58 y GitHub contiene 59.

Cargo.lock está ignorado pese a que el producto es una aplicación binaria. En esta revisión se resolvieron 209 paquetes: otra fecha puede resolver versiones diferentes. Conviene versionar el lock, establecer rust-version/MSRV y usar --locked en CI. Tener edition=2021 no es por sí mismo un defecto: una migración de edition debería hacerse después de revisar la FFI y las APIs de entorno.

Propuesta de CI: fmt --check, clippy --all-targets -- -D warnings y test en Windows/Linux/macOS, con pruebas aisladas. Añadir pruebas de regresión para los errores confirmados y pruebas de integración de CLI con config en un directorio temporal. Revisar la dependencia directa walkdir, que no se utiliza directamente, y las features de git2 si no se necesita SSH/HTTPS; medir tamaño/tiempo antes de cambiar features. No actualizar todas las dependencias simultáneamente durante las reparaciones funcionales.

## Evaluación por módulo

| Módulo | Evaluación y foco |
|---|---|
| main.rs / cli.rs | CLI clara y clap bien elegido. Parsing manual previo con env::args limita soporte a rutas no Unicode, duplica el listado de subcomandos y mezcla filesystem con parsing. Trasladar raíz temporal a un argumento tipado cuando se revise el contrato. open/config --edit son MVP documentados, no bugs inesperados. |
| config.rs | Defaults de Serde y session_roots no serializado son buenas decisiones. Resolver identidad, guardado transaccional, validación de action keys/duplicados y opciones sin consumidor. |
| scanner.rs | Análisis separado del descubrimiento y paralelismo aprovechable. Corregir política de walk, monorepos, identidad, errores y actividad. Result actualmente transmite pocos fallos porque muchos se silencian. |
| snapshot.rs | Reutiliza nombres/metadata y evita muchos exists. No cachea contenido: package.json se lee por varios detectores. Diferenciar directorio ilegible de directorio vacío y conservar errores útiles. |
| detect.rs | Cobertura amplia, pero datos mezclados entre etiquetas de stack y manager y muchas heurísticas textuales. Centralizar parsers y reglas. |
| commands.rs | Sugerir sin ejecutar evita efectos durante scans. Separar comandos inferidos de confirmados y no truncar el modelo a seis elementos. |
| git.rs | Separación fast/status es buena para latencia inicial. Corregir flags y propagación; obtener OID directamente del upstream en vez de reconstruir refs/remotes para admitir también upstream local. Ahead/behind refleja refs locales y no confirma el estado vivo de GitHub sin fetch. |
| health.rs | Score limitado y warnings tipados son útiles. Ajustar semántica, incertidumbre y duplicación; comprobar los resultados reales y no helpers que repiten el algoritmo. |
| project.rs | Enums y Serde son una base clara. Reducir representaciones duplicadas e impedir mutaciones que rompan invariantes. |
| app.rs | Máquina de modos reconocible y generación Git correcta. Hacer la selección estable, sacar I/O del UI thread, cancelar/coalescer workers y eliminar el stub de prioridad o implementarlo. |
| input.rs | Handlers por modo son legibles. Capturar destinatarios, propagar persistencia y actualizar filtros después de mutaciones. |
| tui.rs | Buen uso de Command con argumentos separados. Añadir RAII, política de stdio, resultados de procesos y aislamiento de tests. |
| artifacts.rs | Modelo simple útil. Descubrir targets reales, validar tipos de paths y decidir correctamente si abrir una carpeta o su parent: build output toma parent incluso cuando el artefacto ya es Folder. |
| ports.rs | Funcionalidad útil pero la zona más delicada de memoria/plataformas. Encapsular FFI, mejorar asociación y probar parsers puros. |
| scoring.rs | Algoritmo entendible. Precalcular scores y claves de búsqueda antes de sort, evitando múltiples allocations y lecturas del reloj por comparación. Sumar visitas/opens como u64 para no desbordar u32; usar un now fijo para el ranking. Añadir pruebas de matching Unicode y límites. |
| discover.rs | Candidatos y normalización reutilizables. Compartir política/depth con scanner, corregir ranking y usar fixtures para tests; los skips deberían aplicarse antes de clasificar un directorio como proyecto. |
| ui/mod.rs | Composición sencilla. Conectar config, hacer help scroll efectivo y tratar clean/error/checking de forma distinta. |
| ui/layout.rs | Diseño adaptable con constraints apropiadas. Respetar right_panel y verificar comportamiento real con tamaños pequeños. |
| ui/table.rs | Renderiza solo filas visibles y considera unicode-width. Mantener selección por identidad; probar truncado con graphemes, emojis combinados y anchos mínimos. |
| ui/details.rs | Información rica y funciones auxiliares claras. Falta scroll; el ancho mínimo artificial de 20 puede exceder el disponible. Reutilizar utilidades de truncado con table. |
| ui/footer.rs | Reduce hints según ancho, buena base. Menús completos pueden truncar acciones; permitir navegación/ayuda para encontrarlas y manejar drafts largos. |
| ui/theme.rs | Paleta consistente y aislada. Config.theme todavía no selecciona una paleta; hacer explícito el catálogo o quitar la opción. |

## Orden recomendado de reparación

1. ProjectId estable, selección y destinatarios capturados; persistencia con errores visibles y protección contra conflictos. Estas piezas evitan que las reparaciones posteriores sigan escribiendo datos bajo identidades ambiguas.
2. Encapsular FFI y restauración de terminal. Corregir status Git y defaults de editores; son cambios acotados y verificables.
3. Unificar la política de scanner/discovery: opciones, profundidad, monorepos, skips y reporting de errores. Incorporar los fixtures diagnósticos como regresiones de comportamiento correcto.
4. Reestructurar el scan/enriquecimiento en workers, preservar selección entre batches, actualizar vistas tras mutaciones y conectar las opciones de UI.
5. Mejorar parsers, artefactos y health con reglas explícitas. Precalcular búsqueda/ranking y medir rendimiento antes de optimizar clones o estructuras de datos.
6. Versionar Cargo.lock, aislar tests de entorno, establecer MSRV y añadir CI multiplataforma. La versión de GitHub ya corrige el formato y el lint de disposición de tests observados en el checkout local.

Mantendría ratatui, clap, serde y Rayon. Priorizaría contratos verificables y tipos que expresen invariantes sobre añadir capas, traits o async sin una necesidad concreta.
