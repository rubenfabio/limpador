# ADR-0017 — CI local via hooks do git e tarefa agendada

- **Status:** Aceita (substitui a parte "Execução" do ADR-0011)
- **Data:** 2026-10-17
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-00.06

## Contexto

O ADR-0011 previa CI no GitHub Actions. A conta do GitHub do projeto está impedida de rodar Actions (pendência de cobrança), e o dono do produto decidiu não depender do CI no GitHub: as verificações devem rodar localmente. O projeto é de uso próprio por enquanto, com um único desenvolvedor (humano ou agente) por vez, em Windows.

## Decisão

- **Antes de cada commit** (`.githooks/pre-commit`): formatação, Biome e unitários do que mudou.
- **Antes de cada push** (`.githooks/pre-push` → `pnpm ci`): check + test (inclui os dois e2e e a rastreabilidade) + mutação incremental contra o commit que o remoto já tem. Recusa o push se algo falhar. Push só de documentação pula.
- **Semanal**: tarefa agendada do Windows ("Limpador - mutacao semanal", segundas 09:00, roda assim que possível se o PC estava desligado) executa `pnpm mutation:full` e mostra uma notificação do Windows se alguma meta cair.
- Os hooks são ligados pelo `pnpm install` (`core.hooksPath`).
- O workflow do GitHub continua no repositório, mas **só manual** (`workflow_dispatch`), pronto para ser religado.
- A `main` no GitHub é protegida contra force-push e exclusão, **sem** exigir checks (que não seriam reportados).

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **Hooks + tarefa agendada** | Nenhum serviço externo; bloqueia código quebrado antes de sair da máquina | Push mais lento (~10 min); `--no-verify` permite pular |
| GitHub Actions | Ambiente limpo e independente | Indisponível na conta; minutos e cobrança |
| Só comandos manuais | Push rápido | Fácil esquecer; nada impede código quebrado de subir |
| Exigir checks na proteção da branch | Bloqueio no servidor | Os checks nunca rodariam: merges travados |

## Consequências

**Positivas**
- Funciona offline e sem custo; o mesmo comando (`pnpm ci`) vale para pessoas e agentes.

**Negativas / custos aceitos**
- O ambiente de verificação é a máquina de quem desenvolve, não um ambiente limpo.
- O bloqueio pode ser contornado com `--no-verify`; a regra é não fazer isso para esconder falhas.
- A mutação semanal só roda com o usuário logado (a notificação precisa da sessão).

**O que passa a ser obrigatório**
- Toda verificação nova entra em `tools/steps.mjs` (grupos `check`, `test` ou `ci`), para que hook e comandos manuais rodem o mesmo.
- Para voltar ao CI no GitHub, cria-se um ADR novo que substitua este.
