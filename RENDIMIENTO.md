# Rendimiento de arranque y cierre de ds

> Actualización 2026-10-05: las correcciones de los siete grupos pendientes están documentadas con mediciones por commit en [docs/performance/README.md](docs/performance/README.md). Este análisis conserva las observaciones de su revisión original; la recarga ya funciona en segundo plano, la salud se calcula fuera del hilo de entrada y el dibujo responde a cambios.

Mediciones del 4 de octubre de 2026, Windows x86_64, ejecutable **release instalado**. Este análisis no cambia el código de la aplicación, la configuración, el diseño ni las dependencias.

## Resultado principal

Con las cuatro raíces configuradas y 75 proyectos, el primer cuadro aparece en unos **100 ms**. Pulsar `q` devuelve la terminal en aproximadamente **1,5 ms** y el proceso termina en **8,5 ms**. Hay más margen en el arranque que en el cierre.

## Cómo se midió

- Procesos nuevos, consola Windows ConPTY de 140 × 40 caracteres y captura continua de la salida. El cronómetro empieza antes de `CreateProcessW`. Se detecta la primera pantalla al recibir el pie con `quit`.
- El cierre se mide desde enviar la tecla hasta la terminación del proceso. También se registra la secuencia para abandonar la pantalla alternativa. Se comprobó salida cero y restauración de la terminal en las 30 pruebas de cierre.
- Las pruebas normales envían `q` 250 ms después de la primera pantalla; también se probó enviarla inmediatamente y usar Ctrl+C.
- Una copia temporal del código, compilada en release con contadores, mide las fases internas. Esa copia tiene instrumentación y no sustituye al ejecutable instalado. Sus tiempos externos no se usan para afirmar mejoras.
- Caché del sistema ya utilizada por ejecuciones anteriores. No son mediciones después de reiniciar ni de disco frío. Tampoco incluyen abrir una ventana nueva de Windows Terminal, su pintura en pantalla o el tiempo de un shell.
- La configuración existía y contenía raíces; se evitó el camino que descubre y guarda raíces automáticamente. Su hash permaneció igual después de las pruebas. No se registran nombres de proyectos ni notas en los resultados JSON.
- Las medianas ayudan a separar el comportamiento típico de variaciones de planificación, antivirus y carga del equipo. Las tandas no prueban rendimiento universal.

## Mediciones del ejecutable instalado

| Escenario | Muestras | Mediana | Mínimo–máximo |
| --- | ---: | ---: | ---: |
| Primer cuadro, cuatro raíces / 75 proyectos | 10 | 100,185 ms | 97,769–117,094 ms |
| Primer cuadro, solo este repositorio | 10 | 57,080 ms | 51,795–105,045 ms |
| Proceso `ds --version` completo | 20 | 18,023 ms | 17,328–21,753 ms |
| `q` → proceso terminado, tras 250 ms | 10 | 8,474 ms | 7,761–9,174 ms |
| `q` → terminal restaurada, tras 250 ms | 10 | 1,506 ms | 1,073–1,673 ms |
| `q` inmediato → proceso terminado | 10 | 9,899 ms | 8,829–13,503 ms |
| Ctrl+C → proceso terminado, tras 250 ms | 10 | 7,616 ms | 7,049–34,216 ms |

Las pruebas posteriores de arranque tuvieron medianas de unos 99–113 ms y algunos máximos de 159–166 ms. El valor de `--version` incluye creación y terminación del proceso, argumentos y su salida; no es una medición pura del cargador de Windows.

## Dónde se consume el tiempo

`main` carga la configuración; `run_tui` construye `App`; `App::new` ejecuta `reload` y espera a `scan_roots`. Solo después se activa la terminal y se dibuja el primer cuadro. El estado de cambios Git y los puertos se consultan en segundo plano.

En cinco ejecuciones de la copia instrumentada:

| Fase | Llamadas por ejecución | Mediana del tiempo |
| --- | ---: | ---: |
| Carga y análisis TOML | 1 | 3,465 ms de reloj |
| Escaneo completo | 1 | 58,667 ms de reloj |
| Descubrimiento de directorios | 4 | 76,193 ms acumulados |
| Análisis de proyectos | 75 | 587,633 ms acumulados |
| Información Git inicial | 56 intentos | 538,211 ms acumulados |
| Dentro de Git: abrir repositorio | 56 intentos | 296,743 ms acumulados |
| Dentro de Git: rama | 54 | 57,441 ms acumulados |
| Dentro de Git: último commit | 54 | 67,392 ms acumulados |
| Dentro de Git: upstream | 54 | 63,455 ms acumulados |
| Dentro de Git: ahead/behind | 54 | 21,030 ms acumulados |
| Lectura de snapshots de directorios | 75 | 12,729 ms acumulados |

**Los tiempos acumulados incluyen tareas simultáneas y fases anidadas: no se suman ni equivalen a latencia de arranque.** El trabajo Git representa aproximadamente el 92 % del tiempo acumulado del análisis de proyectos. Esto identifica una prioridad, pero no promete reducir un 92 % el arranque. Dibujar el primer cuadro costó aproximadamente 2,9 ms en la tanda instrumentada anterior.

## El paralelismo ya ayuda

Se cambió `RAYON_NUM_THREADS` únicamente dentro de los procesos de prueba, sin editar variables persistentes ni configuración. Equipo con 16 procesadores lógicos; cinco muestras por caso:

| Hilos | Mediana hasta el primer cuadro |
| --- | ---: |
| Predeterminado, primera tanda | 98,951 ms |
| 1 | 320,233 ms |
| 2 | 200,956 ms |
| 4 | 131,622 ms |
| 8 | 103,142 ms |
| 16 | 99,562 ms |
| Predeterminado, segunda tanda | 112,578 ms |

No hay evidencia para limitar los hilos ni añadir más paralelismo. Ocho y dieciséis están cerca dentro de la variación observada. El trabajo debe centrarse en reducir consultas y lecturas repetidas.

## Mejoras prioritarias que conservan el comportamiento

1. **Reutilizar `HEAD` y el commit dentro de `get_git_info_fast`.** Actualmente rama, último commit, upstream y ahead/behind vuelven a consultar `repo.head()`; además se convierte `HEAD` a commit dos veces. Obtenerlos una vez reduce consultas manteniendo los mismos campos. Verificar ramas normales, HEAD separado, repositorios sin commits y upstream inexistente. La ganancia real debe medirse; abrir los repositorios seguirá siendo necesario y es la parte mayor.
2. **Evitar lecturas repetidas durante un mismo escaneo.** `DirSnapshot` guarda nombres y metadatos, pero `read_to_string` vuelve a abrir el archivo. Las distintas detecciones consultan manifiestos como `package.json`. Se puede reutilizar contenido durante ese escaneo, con límites de tamaño y vida útil. Un caché persistente necesita invalidación correcta y añade más complejidad; no es la primera opción.
3. **Dar presupuesto al procesamiento de resultados de Git.** `poll_hydration_results` consume todos los resultados disponibles y vuelve a leer directorios para recalcular salud, antes de atender la siguiente tecla. Procesar lotes acotados puede mejorar la respuesta cuando llegan muchos resultados juntos. El cierre inmediato medido ya es rápido, pero no representa miles de proyectos ni un disco ocupado. El orden de actualización de filtros y selección necesita verificación.
4. **Salir antes del dibujo siguiente cuando `should_quit` ya está activo.** Hoy `q` provoca un cuadro adicional antes de comprobar esa bandera. Ctrl+C sale directamente. La diferencia típica observada es pequeña; no será una mejora de decenas de milisegundos. Mantener el guard de restauración de terminal.
5. **Redibujar cuando hay cambios.** El bucle actual recalcula la pantalla cada 50 ms aunque no haya novedades. Un indicador de cambios puede reducir CPU y asignaciones en reposo. Debe activarse por teclas, resultados, recarga y redimensionado. Beneficia principalmente el uso continuo, no el arranque.
6. **Simplificar el descubrimiento después de optimizar Git.** El recorrido por raíz es secuencial, pero usa `Arc<Mutex<...>>` y clona colecciones. Se puede evitar parte de esa coordinación sin cambiar los directorios detectados. Su ganancia debe demostrarse; los contadores indican que Git merece atención primero.

## Cambios que requieren otra decisión

Mostrar la TUI antes del escaneo o usar resultados persistidos podría mejorar mucho el primer cuadro percibido, pero introduce una fase de carga o datos temporalmente anteriores. Eso cambia el comportamiento inicial y queda fuera de las optimizaciones internas propuestas aquí. Lo mismo aplica a quitar datos Git, reducir profundidad o dejar de detectar ciertos proyectos.

También se puede experimentar con LTO y ajustes del perfil release en una copia, pero no hay medidas que demuestren una mejora todavía. No se propone añadir dependencias, reemplazar libgit2 ni modificar opciones de Git del usuario.

## Cierre: conclusiones concretas

Salir normalmente no guarda configuración, no une los hilos de Git/puertos y no espera un ciclo fijo de 50 ms: la espera de eventos despierta cuando hay entrada. Los 50 ms son el tiempo máximo de espera en reposo, no una penalización obligatoria para `q`.

La recarga manual sigue siendo síncrona: mientras escanea, ese hilo no atiende teclas. Las pruebas de cierre comienzan después del primer cuadro, no durante una recarga. Tampoco cubren salir de una herramienta externa lanzada en modo terminal, cuyo tiempo depende de esa herramienta.

## Evidencia reproducible

Herramientas y resultados locales en:

`C:\Users\dpahu\.codex\visualizations\2026\10\04\01a10608-4920-79c3-a1a6-d44d4f78570f\ds-audit`

- `bench_conpty.py`: proceso real, captura ConPTY, primera pantalla y cierre.
- `prepare_perf.py`: copia e instrumentación inicial del código para medir fases.
- `performance_results.json`: primera tanda completa.
- `git_performance_results.json`: desglose adicional de las consultas Git.
- `parallelism_performance_results.json`: comparación de hilos.
- `shutdown_performance_results.json`: cierre inmediato, cierre normal y Ctrl+C.

La copia instrumentada quedó fuera del código del proyecto y no se instaló. Los resultados son una línea base para comparar cada optimización futura con las mismas raíces y consola.
