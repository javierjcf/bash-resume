# Documentación de stepwise

| Documento | Para qué |
|---|---|
| [spec.md](spec.md) | **Qué** hace el programa: el comportamiento observable y la fuente de verdad. |
| [architecture.md](architecture.md) | **Cómo** está organizado el código: módulos, imports, tests y errores. |
| [decisions.md](decisions.md) | **Por qué** está hecho así: ADRs cortos, solo se añaden. |
| [tasks.md](tasks.md) | **Qué falta**: tareas con estado y criterio de aceptación. |
| [cheatsheet.md](cheatsheet.md) | Chuleta de comandos de cargo y rustup para jugar. |
| [learning.md](learning.md) | **Qué hemos aprendido**: diario de conceptos de Rust por tarea. |

## Flujo de trabajo

```
spec.md ──► decisions.md ──► tasks.md ──► código + tests ──► learning.md
   ▲                                          │
   └──────── si cambia el comportamiento ─────┘
```

1. Antes de programar, el comportamiento está en `spec.md`. Si una tarea descubre algo nuevo, primero se actualiza la spec.
2. Cuando hay que elegir entre alternativas, se añade una entrada en `decisions.md`.
3. Se trabaja **una tarea a la vez**: se marca `[~]`, se implementa con tests y se marca `[x]` cuando cumple su criterio de aceptación.
4. Al cerrar la tarea, se añade su entrada en `learning.md`.
5. Un commit por tarea (o por paso lógico dentro de ella) siguiendo [Conventional Commits](https://www.conventionalcommits.org/es/):

   ```
   tipo(ámbito): descripción en imperativo, minúsculas, sin punto final

   Cuerpo opcional: qué y por qué (no cómo).

   Refs: T3
   ```

   - **Tipos:** `feat` (funcionalidad), `fix` (corrección), `test`, `docs`, `refactor`, `chore` (andamiaje, deps, config).
   - **Ámbito:** el módulo (`parser`, `state`, `exec`, `menu`…). Se omite si es transversal.
   - **Pie `Refs: Tn`:** enlaza el commit con su tarea de [tasks.md](tasks.md).
   - Ejemplo: `feat(parser): clasificar líneas por prefijo` + `Refs: T1`.
