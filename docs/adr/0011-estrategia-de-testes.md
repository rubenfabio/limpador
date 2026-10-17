# ADR-0011 — Testes primeiro, mutação com metas por criticidade e e2e em duas camadas

- **Status:** Aceita — a parte "Execução" (CI no GitHub Actions) foi substituída pelo [ADR-0017](0017-ci-local.md)
- **Data:** 2026-10-17
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** F00 (T-00.01–T-00.07), todas

## Contexto

O app apaga coisas. Cobertura de linhas não prova que os testes pegariam um erro nas regras que decidem o que apagar. As tarefas já têm critérios de aceite no formato Dado/Quando/Então, prontos para virar testes.

## Decisão

- **Testes primeiro**: cada critério de aceite vira um teste que falha antes da implementação, com o identificador do CA no nome (`tNN_MM_caK`).
- **Unitários e integração**: `cargo test` (núcleo) e Vitest (interface).
- **Mutação**: `cargo-mutants` e StrykerJS. Metas de **≥ 90% em código crítico** (risco, destino, ganho, proteções, vereditos git, revalidação, quarentena, automação, permissões de agentes) e **≥ 70% no resto**. Meta não atingida bloqueia o merge.
- **E2E em duas camadas**: interface com núcleo simulado (Playwright) e app real via WebDriver (`tauri-driver` + WebdriverIO) para as jornadas J1–J9.
- **Execução**: verificações rápidas antes do commit; CI no GitHub Actions em Windows; mutação incremental por PR e completa semanal.

Detalhes em `docs/estrategia-de-testes.md`.

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **Mutação com metas por criticidade** | Esforço concentrado onde o erro apaga dados | Exige manter a lista de módulos críticos |
| Meta única de 80% | Simples | Trata igual o que apaga dados e o que formata texto |
| Só cobertura de linhas | Barato | Não mede a qualidade dos testes |
| E2E só no app real | Fidelidade máxima | Lento e frágil |
| E2E só na interface | Rápido | Não prova a integração |

## Consequências

**Negativas / custos aceitos**
- A mutação é lenta: por isso é incremental no PR.
- O WebDriver do Tauri não roda no macOS; o Mac fica coberto pelas outras camadas quando chegar.

**O que passa a ser obrigatório**
- Uma tarefa só fica `concluída` com todos os CAs cobertos (verificador de rastreabilidade, T-00.07) e as metas de mutação atingidas.
- Mudar a lista de módulos críticos exige justificativa no PR.
