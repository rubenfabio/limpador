# ADR-0020 — TanStack Router e Query na interface; fs4 para espaço em disco

- **Status:** Aceita
- **Data:** 2026-10-17
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** F08 (T-08.01 a T-08.06), T-10.04

## Contexto

A F08 traz várias telas (painel, ranking, visões por projeto e categoria, detalhe, seleção e resumo) que compartilham o resultado da análise e a seleção. Os dados vêm de comandos Tauri e mudam com ações (analisar, limpar, proteger). O painel também precisa do espaço usado e livre de cada disco: a biblioteca padrão do Rust não oferece isso, e a API do Windows exigiria `unsafe`, proibido no projeto.

## Decisão

**Interface**
- **TanStack Router** com rotas definidas em código e histórico em memória (`createMemoryHistory`): o app desktop não tem URLs para compartilhar nem barra de endereço.
- **TanStack Query** para tudo o que vem do núcleo: análise, configurações, quarentena. Ações (analisar, limpar, proteger) são mutações que invalidam as consultas afetadas.
- A seleção de itens é estado de interface (contexto React), não dado do núcleo.

**Núcleo**
- Crate **`fs4`** para espaço total e disponível por unidade (`total_space`, `available_space`), com API segura.

## Alternativas consideradas

| Tema | Alternativa | Por que não |
|---|---|---|
| Interface | React puro, navegação por estado | Mais simples, mas o dono do produto preferiu rotas tipadas e cache de dados |
| Interface | React Router + Zustand | Menos integração entre rotas e dados |
| Disco | sysinfo | Bem maior que o necessário |
| Disco | PowerShell `Get-PSDrive` | Um processo por consulta e saída em texto |

## Consequências

- Testes da interface criam o roteador com histórico em memória e um `QueryClient` sem novas tentativas.
- O e2e da interface continua simulando o núcleo pelo IPC; as consultas do Query passam por ele normalmente.
