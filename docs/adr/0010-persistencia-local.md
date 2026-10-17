# ADR-0010 — Persistência apenas local, na pasta de dados do app

- **Status:** Aceita
- **Data:** 2026-10-16
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-02.05, T-09.04, T-11.01, T-11.02

## Contexto

O app guarda preferências, proteções, o índice da quarentena, o log de ações e os snapshots. É de uso pessoal, roda offline e lida com caminhos e nomes de projetos do usuário.

## Decisão

Tudo fica **na máquina**, na pasta de dados do app do usuário. Nenhum dado é enviado a servidor. O log de ações é somente acréscimo.

O **formato** de armazenamento (arquivos estruturados ou banco embarcado) ainda **não foi decidido** e terá um ADR próprio quando T-02.05 ou T-11.01 começarem.

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **Local** | Privacidade, funciona offline, simples | Sem sincronização entre máquinas |
| Nuvem | Histórico entre máquinas | Desnecessário para uso pessoal; risco de privacidade |

## Consequências

- Nenhuma telemetria sem um ADR próprio e consentimento explícito.
- A pasta de dados é redirecionável apenas em builds de teste (ADR-0012).
