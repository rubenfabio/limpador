# ADR-0021 — SQLite com rusqlite (bundled) para histórico e quarentena

- **Status:** Aceita
- **Data:** 2026-10-18
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-09.04, T-09.05, T-09.06, T-11.01, T-11.02, T-11.03

## Contexto

O ADR-0018 decidiu guardar o histórico (log de ações, snapshots, índice da quarentena) em SQLite, mas deixou a biblioteca e o esquema para quando F09/F11 começassem. O núcleo é síncrono (coletores em threads, ADR-0004) e precisa rodar igual no app, na CLI e no MCP (ADR-0003), sem depender de um SQLite instalado no sistema.

## Decisão

- Usar a crate **`rusqlite`** com a feature **`bundled`** (o SQLite é compilado junto) no núcleo.
- O banco `historico.db` fica na pasta de dados do app; a casca passa o caminho (redirecionável só em sandbox, ADR-0012).
- O esquema tem **versão** (`PRAGMA user_version`) e migrações sequenciais no código, aplicadas ao abrir.
- Toda escrita que muda mais de uma tabela roda numa **transação**.
- O núcleo expõe o banco por uma abstração própria (ex.: índice da quarentena), para que os coletores e a casca não escrevam SQL.

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **rusqlite bundled** | Síncrono como o núcleo, maduro, sem depender do sistema | Compila o SQLite (build um pouco mais lento) |
| sqlx | SQL checado em compilação, assíncrono | Exige runtime assíncrono no núcleo; mais pesado |
| rusqlite + manifesto JSON por pasta da quarentena | Reconstrói o índice se o banco se perder | Duas fontes da verdade para manter em sincronia |

## Consequências

**Positivas**
- Mesmo comportamento no app, na CLI e no MCP; nenhum pré-requisito novo na máquina.

**Negativas / custos aceitos**
- O primeiro build compila o SQLite em C (já há o Build Tools com C++).

**O que passa a ser obrigatório**
- Mudança de esquema só por migração nova; nunca editar uma migração publicada.
- Testes usam bancos em pastas temporárias, nunca o do usuário.

## Referências

- ADR-0010, ADR-0018, ADR-0008.
