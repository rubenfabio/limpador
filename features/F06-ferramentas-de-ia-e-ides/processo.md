# F06 — Coletor de ferramentas de IA e editores

**Fase:** MVP · **Depende de:** F01, F02, F05 (worktrees)

## Objetivo

Encontrar o lixo gerado pelo fluxo de trabalho com IA e pelos editores: worktrees e workspaces de agentes, modelos locais, navegadores de automação, caches de editor e histórico de sessões.

## Fluxo do processo

1. Ler o catálogo configurável de locais conhecidos de cada ferramenta.
2. Detectar quais ferramentas estão instaladas.
3. Para cada ferramenta, listar os itens com tamanho e última utilização.
4. Cruzar com os projetos: worktree de agente ligada ao repositório de origem; workspaceStorage ligado a uma pasta que ainda existe ou não.
5. Gerar os itens; worktrees de agentes usam as regras do coletor Git (F05) e só ganham o rótulo da ferramenta de origem.

## Regras-chave

- Locais de ferramentas de IA mudam com frequência: ficam num catálogo editável, não fixos no código.
- Caches de editor só podem ser limpos com o editor fechado (I5).
- Transcrições e histórico de sessões são conteúdo do usuário: 🔴 e quarentena.

## Ordem das tarefas

T-06.01 → T-06.02 → T-06.03 → T-06.04 → T-06.05

## Definição de pronto

O app mostra, por ferramenta, quanto espaço ocupa e o que pode sair: worktrees de agentes concluídas, modelos não usados, versões de navegador não referenciadas, workspaceStorage órfão e sessões antigas.
