# code-review-tui

Revisor de código en la terminal, con el flujo mental de una *review* de pull
request de GitHub, para los cambios **aún no commiteados** de cualquier repo git.
Escrito en Rust con [ratatui](https://ratatui.rs).

Mostrá el diff del working tree, dejá comentarios anclados a líneas o rangos,
escribí un comentario general, elegí un veredicto **LGTM** / **KO** y, al
finalizar, generá un reporte Markdown estilo PR con todo lo anterior.

## Características

- **Diff de lo no commiteado** contra `HEAD`: staged + unstaged + untracked +
  borrados, en un solo conjunto de cambios (vía `git2`/libgit2).
- **Vista lateral (split) por defecto** con columnas OLD | NEW, o **unificada**
  con la tecla `t`.
- **Lado activo conmutable** en split (`h`/`l` o `←`/`→`): el cursor se resalta
  en la columna elegida y los comentarios se anclan a ese lado.
- **Comentarios** de una línea (`c`) o de un **rango** multilínea (`v`),
  anclados a `archivo:línea` / `archivo:Lini-Lfin`.
- **Resaltado de sintaxis** para PHP y TypeScript.
- **Paneles numerados** y enfocables: `[1]` FILES, `[2]` DIFF, `[3]` hilo de
  comentario.
- **Pantalla final** con el resumen de comentarios, el comentario general y el
  veredicto; al guardar escribe `code-review-<fecha>.md` en el directorio actual.

## Instalación

Requiere un toolchain de Rust (edición 2024; probado con 1.95).

```bash
git clone git@github.com:pauhoms/code-review-tui.git
cd code-review-tui
cargo build --release
```

El binario queda en `target/release/reviewv2`.

## Uso

Ejecutalo dentro de un repo git con cambios sin commitear:

```bash
cd /ruta/a/tu/repo
/ruta/a/code-review-tui/target/release/reviewv2
```

Si no hay cambios, muestra un estado vacío y se sale con `q`. Al finalizar una
review, el reporte Markdown se escribe en el directorio actual.

## Atajos de teclado

| Tecla | Acción |
|---|---|
| `1` / `2` | Enfocar panel FILES / DIFF |
| `Tab` / `Shift+Tab` | Ciclar el foco entre paneles |
| `j` / `k` | Mover (archivo si FILES, línea si DIFF) |
| `h` / `l` · `←` / `→` | Cambiar el lado activo (OLD / NEW) en split |
| `t` | Alternar vista split / unificada |
| `c` | Comentar la línea bajo el cursor |
| `v` | Iniciar selección de rango (`j`/`k` extiende, `c` comenta) |
| `↵` | Abrir el hilo de comentario de la línea |
| `g` | Pantalla final (comentario general + veredicto) |
| `Ctrl+S` | Guardar comentario / finalizar y escribir el reporte |
| `Esc` | Cancelar / volver |
| `q` | Salir |

En la pantalla final: `↑`/`↓` recorre los comentarios, `←`/`→` elige el
veredicto (LGTM / KO) y `↵` salta al hilo del comentario seleccionado.

## Arquitectura

Tres capas desacopladas y testeables:

1. **`diff`** — adquisición del diff no commiteado con `git2` y modelo
   estructurado (archivos → hunks → líneas tipadas con números viejo/nuevo).
2. **`review`** — modelo puro de la review (comentarios, general, veredicto) y
   su serialización a Markdown determinista, separada de la escritura a disco.
3. **`app`** — la TUI ratatui que orquesta las dos anteriores; todo el estado y
   el manejo de eventos es dirigible *headless* para poder testearlo.

## Tests

```bash
cargo test
```

La TUI se prueba sin terminal real con `ratatui::backend::TestBackend`
(render a un buffer de celdas) inyectando eventos de teclado; las capas de diff
y review se prueban con repos git temporales y comparación de strings.
