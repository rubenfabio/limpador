# ADR-0015 — Integração com agentes via CLI e MCP, com pedidos de aprovação

- **Status:** Aceita (implementação na Fase 2)
- **Data:** 2026-10-16
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** F12

## Contexto

O diferencial do produto é atacar a causa: agentes de IA criam worktrees, branches e caches. Mas dar a um agente o poder de apagar coisas do usuário é um risco sério, e instruções maliciosas podem chegar ao agente por conteúdo externo.

## Decisão

- Os agentes acessam o núcleo por uma **CLI** e por um **servidor MCP**, ambos cascas finas (ADR-0003).
- Um agente só executa diretamente itens **elegíveis para automação** (🟢). Qualquer outro risco vira um **pedido de aprovação**, que o usuário confirma no app pelo fluxo normal de confirmação.
- Agentes **não têm acesso** a preferências, proteções, purga da quarentena nem à lixeira.
- Toda ação registra a origem `agente: <nome>`.

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **Agentes limitados a 🟢 + pedidos** | Útil e seguro; o humano decide o que é arriscado | Exige uma tela de pedidos |
| Acesso total para agentes | Máxima autonomia | Risco inaceitável de perda de dados |
| Só leitura para agentes | Seguro | Não resolve a causa |

## Consequências

- As permissões de agentes são **código crítico** (ADR-0011).
- Adicionar uma ferramenta MCP que altere estado exige revisão contra este ADR.
