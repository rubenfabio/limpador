# ADR-0018 — Configurações em TOML e histórico em SQLite

- **Status:** Aceita
- **Data:** 2026-10-17
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-02.02, T-02.04, T-02.05, T-09.04, T-11.01, T-11.02, T-11.03

## Contexto

O ADR-0010 definiu que tudo fica na máquina, mas deixou o formato em aberto. Há dois tipos de dado: **configurações** (preferências, pastas raiz, exclusões, proteções), que são pequenas e às vezes editadas à mão; e o que **cresce com o tempo** (log de ações, snapshots de análise, índice da quarentena), que precisa de filtros, retenção e gravação atômica.

## Decisão

- **Configurações em TOML**, num arquivo na pasta de configuração do app (`config.toml`), com seções por assunto. Gravação atômica (arquivo temporário + renomear). Campos ausentes assumem o padrão; valores fora da faixa são recusados ao salvar e, ao carregar, substituídos pelo padrão com aviso.
- **Histórico em SQLite** (log de ações, snapshots, índice da quarentena), num arquivo `historico.db` na pasta de dados do app. Biblioteca e esquema são definidos quando F09/F11 começarem.
- As pastas vêm da casca (Tauri) e podem ser redirecionadas só em builds de teste (ADR-0012).
- Leitura e escrita com `serde` (+ `toml`); dependências do núcleo, sem tocar a interface.

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **TOML + SQLite** | Configuração legível e comentável; histórico consultável | Duas tecnologias |
| JSON para configuração | Mesmo formato da interface | Sem comentários, menos amigável à mão |
| Tudo em SQLite | Uma só tecnologia | Configuração ilegível à mão |
| JSON Lines para histórico | Simples | Filtrar e reter exige ler tudo |

## Consequências

- O núcleo ganha as dependências `serde` e `toml` agora, e uma biblioteca SQLite em F09/F11.
- Mudar o formato depois exige migração e um ADR novo.
