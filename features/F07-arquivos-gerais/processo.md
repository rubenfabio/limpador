# F07 — Coletor de arquivos gerais

**Fase:** MVP · **Depende de:** F01, F02

## Objetivo

Categoria separada para o que não é de desenvolvimento, mas pesa: Downloads (o maior item da máquina do usuário, com 17,8 GB), temporários, crash dumps e a lixeira do sistema.

## Fluxo do processo

1. Classificar os arquivos de Downloads por tipo e idade.
2. Medir temporários e crash dumps elegíveis.
3. Medir a lixeira do sistema.
4. Gerar os itens na categoria "Arquivos gerais", visualmente separada das categorias de dev.

## Regras-chave

- Arquivos pessoais são conteúdo do usuário: 🔴 e quarentena (I9).
- A lixeira do sistema nunca é esvaziada automaticamente; esvaziar é sempre uma ação manual confirmada.
- Arquivos em uso são ⛔ (I5).

## Ordem das tarefas

T-07.01 → T-07.02 → T-07.03

## Definição de pronto

O app separa, em Downloads, instaladores e compactados antigos de arquivos pessoais e limpa temporários e crash dumps sem tocar em arquivos em uso.
