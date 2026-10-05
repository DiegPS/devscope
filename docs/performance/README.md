# Correcciones y velocidad por punto — 2026-10-05

Se estableció la base antes de tocar producción, en `5033a50`. Cada grupo pasó pruebas y benchmark antes de su commit; los JSON contienen todas las muestras conservadas. No se añadió ninguna dependencia ni se cambió Cargo.lock o el diseño.

## Muestra y método

Windows, Rust 1.95, perfil release. Fixture aislada bajo target/: **1.000 proyectos, 200 repositorios Git y 16.000 archivos fuente**, con cuatro ecosistemas, carpetas de dependencias excluidas y cambios Git sin commit. Cada medición realiza tres calentamientos y veinte muestras, y comprueba que las veinte devuelvan exactamente 1.000 proyectos. La compilación queda fuera del cronómetro. Son tiempos con cachés calientes: no representan disco frío ni todos los proyectos personales.

`scan_ms` incluye detección y Git básico; `full_scan_ms` añade Git completo y salud final. El orden, carga del equipo, antivirus, cachés y planificación introducen variación: cambios pequeños no demuestran causalidad. En el punto 6 una tanda dio 323 ms y otra 283 ms sin cambiar código; se conservó la repetición aislada. No se aplica un umbral absoluto universal de milisegundos.

## Comparación de cada punto

Medianas y p95 en ms. La última columna compara el escaneo con el punto inmediatamente anterior.

| Versión | Escaneo mediana | Escaneo p95 | Con Git completo mediana | Diferencia escaneo |
| --- | ---: | ---: | ---: | ---: |
| baseline | 268.78 | 287.26 | 434.05 | — |
| point-1 | 266.42 | 286.25 | 433.29 | -0.9% |
| point-2 | 267.19 | 289.93 | 436.44 | +0.3% |
| point-3 | 294.86 | 322.17 | 463.32 | +10.4% |
| point-4 | 301.02 | 324.74 | 471.56 | +2.1% |
| point-5 | 295.26 | 309.06 | 460.33 | -1.9% |
| point-6 | 282.93 | 296.74 | 441.74 | -4.2% |
| point-7 | 282.41 | 292.15 | 450.21 | -0.2% |

La mediana final del escaneo es aproximadamente **5% mayor que la base** y la de Git completo aproximadamente **4% mayor**. Las comprobaciones de fuentes y las reglas de recorrido hacen trabajo que antes faltaba. No se presenta esta serie como una mejora de velocidad del scanner. El primer intento secuencial del punto 2 dio 976 ms; se descartó antes del commit y se corrigió con recorrido paralelo y sin canonicalización redundante de cada proyecto.

## Cambios, pruebas y commits

1. **Identidad y persistencia** — `7e56357`: rutas canónicas y deduplicación; migración conservadora de notas, estados e historial; bloqueo de archivo y combinación de cambios simultáneos. Ediciones incompatibles fallan sin sobrescribir datos. Los aliases antiguos se conservan; los counters de aliases históricos usan máximos, no sumas que inflen el historial. La última tanda comprueba también doce escritores reales y dos primeras escrituras sobre un historial migrado.
2. **Recorrido y discovery** — `2fadfba`: scan_hidden, respect_gitignore y follow_symlinks; raíz a profundidad cero, monorepos sin podar miembros, política compartida y errores propagados. La suite Unix comprueba symlinks, duplicación del destino y ciclos.
3. **Actividad y artefactos Cargo** — `acc28e5`: fechas de archivos en src/lib/app/tests/scripts, sin seguir symlinks ni entrar en caches de compilación; lectura de package/bin, workspace y target-dir para salidas debug/release. Sin procesos Cargo durante el scanner. Se reutiliza contenido de manifests durante una pasada. La tanda final comprueba que ds no produzca un ejecutable fantasma llamado devscope.
4. **Detección y comandos** — `99d603d`: dependencias declaradas en JSON/TOML/requirements, sin confundir comentarios/descripciones con frameworks; comandos completos y Node install sin scripts. Terminal por plataforma: Windows/macOS/Linux.
5. **Respuesta de TUI** — `fd77f0f`: recarga en un worker con peticiones repetidas combinadas; lista visible conservada, metadatos recientes aplicados al resultado, salud en el worker Git y cancelación al salir/reemplazar trabajo. Canal acotado y dibujo por eventos. El trabajo de fondo inicial comienza después del primer cuadro.
6. **Salud** — `76cfae4`: ramas de trabajo, cambios locales y commits ahead siguen visibles, pero no descuentan salud. Los env ignorados no penalizan; los ya versionados siguen avisando. No se leen secretos. Archivos ejemplo/plantilla se distinguen de los de entorno real.
7. **Contratos y mantenimiento** — último commit de esta serie: estados de config tipados con el TOML anterior conservado, validación de acciones, actualización conjunta de warnings/health, MSRV 1.89 y CI específico; presupuesto de hasta 64 resultados Git por vuelta. Un HEAD/commit compartido para campos Git y upstreams locales/remotos. Launcher real con argumentos que contienen espacios, cwd, env, error de spawn, hijo terminal correcto/fallido y restauración de la TUI.

## Apertura y salida durante recarga

La misma fixture release, ConPTY real, dos calentamientos y diez muestras. Incluye harness de tests y preparación de PTY; no es exclusivamente tiempo de dibujo. La comparación comienza justo antes del punto 5, tras las correcciones de detección.

| Mediana | Antes del punto 5 | Final |
| --- | ---: | ---: |
| Primer cuadro | 338.38 ms | 335.19 ms |
| r seguido inmediatamente de q, hasta salir | 321.74 ms | 9.56 ms |

**Salir durante una recarga mejora aproximadamente 97%.** La apertura permanece comparable; una diferencia de unos 3 ms no demuestra una mejora. El primer cuadro sigue mostrando el escaneo inicial terminado; no se introducen resultados persistidos ni se omiten datos para aparentar velocidad.

## Verificación final

- Format y clippy all-targets sin warnings.
- 151 pruebas unitarias/de contratos y tres CLI: **154 normales** en Windows. Los tres helpers ignored se ejecutan explícitamente en scripts/tests cuando corresponden.
- **11 escenarios PTY**, incluidos lanzar un proceso real, su fallo, recargar y salir.
- Rust 1.89: las mismas pruebas pasan; CI añade Linux en ese MSRV, además de stable Windows/Linux/macOS.
- Cobertura LLVM de unit/CLI/PTY: **84,31% (5.723/6.788 líneas)**; gate obligatorio 75%. El porcentaje incluye tests inline y excluye regression_tests.rs y tests/, como explica TESTING.md.
- Build release verificada. Los tests no escriben configuración personal ni abren editores externos.

## Repetir las mediciones

```powershell
python scripts/bench_scan.py nombre-de-version
python scripts/bench_tui.py tui-nombre-de-version
```

Los scripts guardan resultados en target/bench-reports/. Para comparaciones posteriores, guardar los JSON antes de la modificación y repetir con el mismo conjunto, toolchain y equipo. Los perfiles Cargo personalizados y detección exhaustiva de todos los triples/targets no se infieren mediante cargo metadata: no se ejecuta el build system de proyectos ajenos durante el escaneo.

## Verificacion adicional del ejecutable de produccion

Se conservaron el ejecutable instalado original y el release de d169c0b y se compararon diez pares, alternando el orden A/B, en la misma fixture. La configuracion personal mantuvo el mismo hash. Esta comprobacion usa los binarios reales y evita mezclar medidas del harness con produccion.

| Mediana | Original instalado | Release d169c0b |
| --- | ---: | ---: |
| Primer cuadro, caso q | 305,51 ms | 329,25 ms |
| q hasta terminar proceso | 15,63 ms | 7,92 ms |
| r seguido de q hasta terminar | 292,73 ms | 8,62 ms |

Respecto al ejecutable original, **abrir es aproximadamente 24 ms mas lento** en esta muestra grande; cerrar normalmente es aproximadamente 8 ms mas rapido y cerrar durante recarga mejora aproximadamente 97%. El coste de revisar fuentes reales no se oculta tras la comparacion anterior, que comenzaba justo antes del punto 5. Los hashes y muestras estan en production-paired.json.

## Correccion encontrada por CI de macOS

La prueba de env versionado detecto una diferencia entre /var y /private/var en temporales de macOS. Se canonicaliza el directorio de trabajo antes de calcular el path del indice, manteniendo intacto el nombre del archivo env. Se conserva el test y se agrega un contrato Unix con un alias symlink a un repositorio. ci-macos-env-path.json guarda la comparacion de velocidad de esta reparacion adicional. La rama de env no se ejecuta en la fixture de velocidad; las diferencias pequenas entre tandas no se atribuyen a ese ajuste.
