# Auditoría y correcciones de diseño de Devscope / ds

Fecha: 2026-10-04. Base del diagnóstico: `db65a68`. Alcance: diseño visual, información, navegación por teclado, adaptación al tamaño del terminal y estados de la TUI.

## Correcciones implementadas

Revisión de densidad: se recuperó el encabezado de una sola fila y la división vertical original 56/44. Projects muestra filtro, orden, vista y consulta activa sobre su propio borde, sin reservar una segunda fila. En 80×24 vuelven a verse nueve proyectos, igual que en la versión original. Desde 125 columnas la lista conserva toda la altura mediante paneles laterales. El nombre aparece una sola vez dentro de detalles; se ajusta en varias líneas si es largo. Se retiraron el campo Name redundante y el título Overview.

- Detalles con foco mediante Tab, desplazamiento con flechas/j/k, PageUp/PageDown y Home/End, contador de líneas y resumen del proyecto fijo. Rutas, notas, ramas, comandos y avisos largos se distribuyen en líneas completas; se puede alcanzar el contenido restante. Cambiar de proyecto reinicia el desplazamiento.
- Ayuda con tamaño limitado por la ventana, desplazamiento real e instrucciones de cierre visibles.
- Consulta y filtro activos visibles después de confirmar la búsqueda. Los estados vacíos distinguen ausencia de proyectos de ausencia de coincidencias y ofrecen pasos de recuperación.
- Edición de notas/búsqueda con una ventana horizontal que sigue el extremo de inserción; cursor e instrucciones permanecen visibles.
- Menús de apertura, configuración y estado en paneles adaptables. Apertura/configuración admiten flechas y Enter además de sus teclas originales; se puede llegar al final en ventanas bajas.
- Mensajes con severidad explícita (Info/Warning/Error), color correspondiente y texto distribuido en líneas. La navegación no los descarta.
- Etiquetas separadas de valores; Git/comandos/puertos antes de la lista completa de salud; sin límite visual de seis avisos. Menor duplicación entre encabezado y tabla, títulos completos de columnas y acciones prioritarias en el pie.
- Indicadores de Git/salud/estado conservan su color en la selección. Ausencia de Git y proyectos archivados reciben tratamiento neutral; etiquetas secundarias más legibles.
- Distribución según ancho y alto: se conserva el umbral lateral original de 125 columnas para priorizar la densidad de la lista; en ventanas muy bajas se usan paneles laterales desde 110 columnas. El cambio de orientación al cruzar el umbral es intencional.
- Preferencias existentes aplicadas: `right_panel=false` inicia en compacto, `show_icons=false` usa indicadores de texto y `[ui] theme="light"` ofrece colores para un terminal con fondo claro. `default`, `dark` y valores desconocidos usan la paleta oscura. El fondo sigue perteneciendo al terminal; no se detecta automáticamente.

No se modificaron el escáner, sus dependencias ni el manifiesto Cargo. Tampoco se modificó la configuración personal durante las verificaciones.

## Capturas y validación final

Galería antes/después, con 24 proyectos de prueba realistas, nombres/rutas/ramas largas, varios stacks, notas extensas y distintos estados de Git/salud:

[Capturas y comparación](docs/design/README.md). La [galería HTML](docs/design/index.html) se abre localmente en un navegador; GitHub muestra el archivo como código.

Las imágenes reconstruyen las celdas y estilos del renderer real. Los datos son fixtures, no datos personales. Se conservan ambas versiones y el mismo conjunto de datos para comparar; además hay capturas del final de ayuda/detalles, errores y tema claro. La fuente de las capturas puede diferir de la fuente del terminal instalado.

Validación local: 138 pruebas normales (135 unitarias/de contratos + 3 CLI), siete escenarios de terminal real, formato y Clippy sin advertencias, build release. Dos pruebas específicas comprueban las nueve filas originales en 80×24 y la ausencia del campo Name duplicado. Cobertura de líneas instrumentadas: 81,41 % (5.037/6.187), por encima del mínimo del 75 %. El alcance y las limitaciones del indicador siguen descritos en TESTING.md.

Comparación de diez ejecuciones alternadas por versión, primera revisión frente a revisión compacta, con la misma configuración real y caché del sistema ya utilizada: escaneo mediano 60 ms / 62 ms; primera pantalla 108,126 ms / 103,683 ms; cierre de proceso 8,123 ms / 6,261 ms. Las variaciones no demuestran mejoras o regresiones causales ni un resultado de arranque en frío. [Datos crudos](docs/design/performance.json).

## Diagnóstico original (antes de las correcciones)

La estructura de lista + detalles es apropiada para explorar proyectos con rapidez. La selección se distingue, las columnas mantienen alineación y existe una identidad consistente con acento turquesa, títulos amarillos y bordes discretos. Conviene conservar esta base.

El principal problema es la accesibilidad de la información: varios contenidos se recortan sin una forma de recuperarlos, y ciertos estados activos dejan de ser visibles. Antes de cambiar la estética, conviene resolver esos problemas y mejorar la jerarquía del panel de detalles.

## Evidencia y límites

Se revisaron `src/ui/`, `src/input.rs`, los estados de `src/app.rs` y las preferencias de `src/config.rs`. Se generaron 14 capturas mediante el renderer real de ratatui con TestBackend, usando proyectos de prueba y una copia externa del código. Se inspeccionaron sus imágenes y el texto de las celdas.

Escenarios: vista detallada en 80×24, 100×30, 124×30, 125×30 y 160×45; compacta en 100×30; ayuda en 80×24 y 160×45; menú de apertura, edición de nota larga, búsqueda, búsqueda confirmada, cero resultados y cero proyectos. Los menús de estado/configuración se revisaron en código, sin una captura propia.

Las capturas usan un fondo oscuro de referencia y Consolas. La aplicación hereda el fondo del terminal; no se verificaron todos los temas, fuentes ni emuladores. Los glifos de las imágenes dependen de la fuente usada para reconstruir las celdas: una caja en estas imágenes no demuestra que falle en Windows Terminal. No se hicieron pruebas con usuarios ni una certificación de accesibilidad. Las prioridades siguientes son un juicio de diseño apoyado en el comportamiento observado.

Galería local fuera del repositorio: `C:/Users/dpahu/.codex/visualizations/2026/10/04/01a10608-4920-79c3-a1a6-d44d4f78570f/design-audit/index.html`. Contiene las capturas actuales, no un rediseño.

## Hallazgos prioritarios

| Prioridad | Hallazgo comprobado | Consecuencia | Dirección propuesta |
|---|---|---|---|
| Alta | Detalles usa un Paragraph sin desplazamiento; todas las secciones se apilan. En 80×24 llega hasta Manager; en 125×30 se corta dentro de Git; incluso en 160×45 los valores de Ports quedan debajo del panel con el proyecto de prueba. | Información ya detectada queda fuera de alcance. | Desplazamiento del panel con foco visible y señal de contenido restante; mantener un resumen del proyecto. |
| Alta | La ayuda ocupa el 60% del ancho y 70% del alto. `help_scroll` cambia con las teclas, pero el renderer no lo utiliza. | En 80×24 solo se ven navegación y las primeras acciones; los atajos restantes y la indicación de cierre quedan ocultos. | Tamaño limitado por contenido y espacio disponible, scroll real e instrucciones de cierre siempre visibles. |
| Alta | Tras Enter en búsqueda, el texto se conserva en `search_query`, pero desaparece del pie. El encabezado tampoco muestra la consulta. Por debajo de 92 columnas tampoco muestra el filtro activo. | Una lista reducida puede parecer incompleta sin explicar por qué. | Mantener consulta y filtro activos visibles; indicar cómo limpiarlos. |
| Alta | Notas y búsquedas se dibujan en una sola línea sin ventana horizontal de edición. | En una nota larga desaparecen el cursor simulado y las instrucciones; se sigue escribiendo fuera de la zona visible. | Hacer que el texto visible siga el punto de edición y reservar espacio para instrucciones. |
| Alta | Menús de apertura/configuración y selector de estado se dibujan enteros en una línea. En 80×24 el menú de apertura de prueba pierde acciones e incluso Esc cancel. | Las opciones siguen funcionando, pero no se pueden descubrir todas. | Menú adaptable de varias filas o panel contextual, conservando las teclas existentes. |
| Media | Errores y éxitos comparten `status_message`, y el encabezado los pinta con `theme.active` verde. Cualquier tecla borra el mensaje. | Un fallo puede parecer un éxito y desaparecer antes de leerse. | Severidad explícita, texto/símbolo además del color, espacio legible para mensajes. |
| Media | El mensaje de lista vacía vive en una sola celda de la primera columna. En 100×30 aparece «No projects match the current». Se usa el mismo mensaje sin proyectos y sin coincidencias. | El diagnóstico se corta y no ofrece una recuperación concreta. | Estado vacío a ancho completo; distinguir configurar raíces de limpiar búsqueda/filtro. |
| Media | A 124 columnas la división es vertical; a 125 cambia a horizontal 60/40. La tabla pasa de un ancho aproximado de 124 a 75 y pierde columnas. La altura no interviene en esa decisión. | Un incremento mínimo del ancho reorganiza la pantalla y reduce información de la lista. | Decidir con ancho mínimo de cada panel y altura disponible; definir un comportamiento estable para ventanas bajas. |
| Media | El pie normal elimina acciones como open, note y status en anchos reducidos; en vista detallada vertical no las considera aunque sobren algunas columnas. | Acciones importantes se vuelven difíciles de descubrir. | Priorizar abrir y buscar según el propósito de la app; agrupar el resto con una indicación visible de ayuda/acciones. |
| Media | En 125×30, la etiqueta «Last active» queda pegada al valor: «Last activeunknown». El ancho fijo de etiquetas no garantiza separación. | Cuesta distinguir etiqueta y dato. | Garantizar al menos un espacio entre ambas y ajustar etiquetas al ancho real. |
| Media | Salud aparece antes de Git/comandos y puede consumir varias filas con advertencias y positivos. «… and N more» no se puede expandir. | La evaluación desplaza datos operativos y presenta una lista incompleta sin acceso al resto. | Resumen compacto de salud y acceso al contenido completo; evaluar Git/puertos/comandos más arriba. |
| Media | La fila seleccionada uniforma colores de stack, Git y salud. Archived usa rojo, igual que estados malos; ausencia de Git recibe verde en la tabla. | Se debilita la distinción entre selección, información neutral y problemas. | Conservar indicadores semánticos legibles sobre selección; reservar rojo para errores/riesgos y tratar ausencia/archivo como neutrales. |
| Baja | Contador, orden y vista se repiten entre encabezado y título de tabla. Hay muchas secciones y separadores, además de colores específicos por tecnología. | La información secundaria compite con el proyecto seleccionado. | Asignar un lugar a cada dato; reducir repetición antes de añadir decoración. |
| Baja | Las abreviaturas H y Act requieren aprendizaje; la paleta fija usa etiquetas grises sobre un fondo heredado. `theme`, `show_icons` y `right_panel` no gobiernan el renderer actual. | La legibilidad depende del terminal y algunas preferencias crean expectativas que no se cumplen. | Explicar abreviaturas, verificar fondos claros/oscuros y decidir explícitamente el alcance de preferencias existentes. |

## Dirección de diseño recomendada

1. **Proyecto primero.** Nombre y selección como ancla; stack y actividad como apoyo. Orden, filtros y consulta activos visibles sin repetirlos.
2. **Detalles útiles antes que extensos.** Resumen breve con estado, Git y salud; organizar el resto para acceder a comandos, puertos, notas y artefactos sin exigir una ventana enorme. El orden exacto es una propuesta que debe validarse, no una conclusión de pruebas con usuarios.
3. **Modos reconocibles.** Buscar, editar y elegir acciones deben indicar claramente qué se está haciendo, sobre qué proyecto y cómo aceptar/cancelar. Mantener las teclas actuales.
4. **Color con significado.** Conservar turquesa como acento, usar amarillo para atención y rojo para problemas, y evitar que selección borre toda la semántica. Verificar los contrastes sobre el fondo efectivo del terminal.
5. **Adaptación predecible.** Definir primero qué debe funcionar en 80×24; ampliar información al crecer la ventana. No prometer que la vista completa cabe en cualquier tamaño: asegurar acceso al contenido restante.

## Orden de trabajo

**Primera tanda: legibilidad y acceso.** Ayuda desplazable, detalles accesibles, contexto persistente de búsqueda, estado vacío completo, edición larga visible y menús que no oculten opciones.

**Segunda tanda: jerarquía y consistencia.** Separación de etiquetas, prioridad de acciones en el pie, mensajes según severidad, reducción de duplicación y revisión del uso de colores.

**Tercera tanda: composición.** Reordenar detalles y ajustar divisiones según ancho/alto, comparando pantallas antes/después. Resolver las preferencias visuales requiere definir su comportamiento, porque activarlas modifica funcionalidades existentes.

Estas tandas describen la propuesta del diagnóstico original; las correcciones aplicadas están enumeradas al inicio del documento.

## Validación cuando se implementen

- Comprobar 80×24, 100×30, 124×30, 125×30 y 160×45, además de ventanas muy bajas, sin limitarse a que el renderer no falle.
- Confirmar que se puede alcanzar el final de ayuda y detalles, que las instrucciones de cierre siguen visibles y que todos los menús ofrecen acceso a sus acciones.
- Confirmar búsqueda activa tras Enter, limpieza con Esc, cero coincidencias, cero proyectos, notas largas y errores de persistencia/apertura.
- Conservar identidad del proyecto seleccionado y atajos existentes al cambiar tamaño, filtro o modo.
- Añadir comprobaciones de contenido y navegación para los problemas concretos; evitar capturas rígidas de cada celda como único criterio de calidad.
- Verificar al menos un terminal real con fondos claro/oscuro y fuentes comunes; las capturas de TestBackend no sustituyen esa comprobación.
- Comparar tiempos de primera pantalla y redibujado antes/después bajo las mismas condiciones. No añadir lecturas de disco, escaneos ni detección de procesos dentro del renderer; mantener las dependencias actuales.

La revisión visual y la medición de rendimiento son verificaciones distintas; las cifras de esta implementación aparecen al inicio del documento.
