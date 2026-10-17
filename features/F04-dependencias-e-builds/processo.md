# F04 — Coletor de dependências e builds

**Fase:** MVP · **Depende de:** F01, F02

## Objetivo

Encontrar dependências instaladas, saídas de build, caches globais de gerenciadores de pacote e runtimes não usados, em todos os ecossistemas comuns.

## Fluxo do processo

1. Percorrer os projetos identificados (F02) procurando pastas de dependência e de build.
2. Medir os caches globais nos locais do catálogo.
3. Detectar versões de runtime instaladas e quais versões os projetos pedem.
4. Gerar os itens: os de projeto herdam a atividade do projeto; os globais não têm projeto.
5. Na limpeza: preferir o comando oficial do gerenciador quando ele existir (ex.: limpeza de cache do próprio gerenciador); caso contrário, remover a pasta.

## Regras-chave

- Só são candidatas as saídas de build **ignoradas pelo git**; uma pasta `dist` versionada nunca é candidata.
- Junções e links dentro de `node_modules` nunca são seguidos (I3).
- Itens de projeto ativo aparecem, mas com ganho seguro reduzido (regra de F01).
- Cada item informa o comando que o recria (ex.: "rode `pnpm install`").

## Ordem das tarefas

T-04.01 → T-04.02 → T-04.03 → T-04.04 → T-04.05 → T-04.06

## Definição de pronto

O app lista e remove `node_modules`, saídas de build e caches globais de JS, Python, Go, Rust, Java e .NET, mostrando para cada item como recriá-lo.
