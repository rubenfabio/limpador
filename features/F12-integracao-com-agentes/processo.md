# F12 — Integração com agentes de IA

**Fase:** 2 (após o MVP) · **Depende de:** F01, F05, F06, F09

## Objetivo

Atacar a **causa** do acúmulo: deixar os agentes de IA consultarem o Limpador e limparem o que eles mesmos criaram, quando a tarefa termina.

## Fluxo do processo

1. O núcleo é exposto por uma interface programática local (linha de comando), sem depender da interface gráfica.
2. Um servidor MCP oferece ferramentas para agentes: analisar, listar e limpar.
3. Um evento de fim de tarefa (ex.: hook do agente) avisa o Limpador que uma worktree ou branch terminou.
4. O Limpador registra a origem dos itens criados por agentes, para classificá-los com mais confiança.

## Regras-chave

- Um agente só pode executar diretamente itens 🟢. Qualquer outro risco gera um **pedido de aprovação** que o usuário confirma no app.
- O agente nunca pode alterar configurações, proteções ou a quarentena.
- Todas as ações vindas de agentes ficam registradas com a origem "agente: <nome>".

## Ordem das tarefas

T-12.01 → T-12.04 → T-12.02 → T-12.03

## Definição de pronto

Ao fim de uma tarefa de um agente, a worktree e a branch dele, se mergeadas, viram um pedido de limpeza que o usuário aprova com um clique, e os caches 🟢 gerados são limpos sozinhos.
