# ADR-0016 — Estrutura do repositório e ferramentas de desenvolvimento

- **Status:** Aceita
- **Data:** 2026-10-17
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-00.01, T-00.06

## Contexto

A T-00.01 cria o esqueleto do projeto. O ADR-0003 exige um núcleo independente da interface. Era preciso escolher o gerenciador de pacotes JS, a ferramenta de lint e formatação do TypeScript e a forma de expor um comando único para testes e verificações.

## Decisão

**Estrutura**

```
Cargo.toml            workspace Cargo (crates/core, src-tauri)
crates/core/          limpador-core — núcleo, sem dependência de Tauri ou da interface
src-tauri/            limpador-app — casca Tauri (comandos finos que chamam o núcleo)
ui/                   interface React + TypeScript (Vite, Vitest)
tools/                scripts de verificação do repositório e seus testes
package.json          workspace pnpm + comandos únicos
```

**Ferramentas**

| Tema | Escolha |
|---|---|
| Gerenciador de pacotes JS | **pnpm** (workspace com `ui` e `tools`) |
| Lint e formatação TS/JSON | **Biome** (indentação de 2 espaços, linha de 100) |
| Lint e formatação Rust | `rustfmt` + `clippy` com `-D warnings`; toolchain fixada em `rust-toolchain.toml` |
| Comando único | Scripts no `package.json` da raiz: `pnpm test`, `pnpm check`, `pnpm fmt`, `pnpm dev`, `pnpm build` |
| Orquestração | `tools/run.mjs` roda **todas** as etapas, mesmo após falha, mostra um resumo e sai com erro se alguma falhar |

## Alternativas consideradas

| Tema | Alternativa | Por que não |
|---|---|---|
| Pacotes JS | npm | pnpm já estava instalado, é mais rápido e economiza disco |
| Lint TS | ESLint + Prettier | Mais dependências e configurações para o mesmo resultado |
| Comando único | `just` | Mais um binário para instalar |
| Comando único | `cargo xtask` | Pior para orquestrar as ferramentas JS |

## Consequências

**O que passa a ser obrigatório**
- `crates/core` nunca depende de `tauri*` nem de `limpador-app` (verificado por teste, `t00_01_ca3`).
- Toda verificação nova entra como etapa em `tools/steps.mjs`, para que local e CI rodem os mesmos comandos.
- Código precisa passar em `pnpm check` (formatação, lint e tipos) antes do commit.
