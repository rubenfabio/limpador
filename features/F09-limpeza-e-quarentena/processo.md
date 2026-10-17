# F09 — Execução da limpeza e quarentena

**Fase:** MVP · **Depende de:** F01, F08 (seleção), coletores

## Objetivo

Executar a limpeza com segurança: confirmar, revalidar, remover ou mover para a quarentena, reportar, e permitir restaurar até a quarentena expirar.

## Fluxo do processo

1. O usuário confirma a seleção (com confirmação reforçada para 🔴).
2. Para cada item, o app **revalida** o estado (I6). Se mudou, o item é pulado com motivo.
3. O destino é decidido: **exclusão direta** (recriável) ou **quarentena** (conteúdo do usuário ou 🔴, I9).
4. Cada coletor executa a ação do seu item.
5. Falhas são isoladas: um item que falha não interrompe os outros.
6. Um relatório final mostra o que foi limpo, o que foi para a quarentena, o que falhou e o espaço liberado agora e na purga.
7. Tudo é registrado no log (I7).
8. A quarentena expira após 14 dias (configurável) e então é purgada; até lá, qualquer item pode ser restaurado.

## Regras-chave

- A quarentena fica no mesmo disco do item, para que mover seja rápido. O espaço só é liberado na purga, e a interface deixa isso claro.
- Restaurar devolve o item ao estado e ao local originais. Se o local estiver ocupado, o app pergunta o que fazer.
- O usuário pode purgar antes do prazo, com confirmação.

## Ordem das tarefas

T-09.01 → T-09.02 → T-09.03 → T-09.04 → T-09.05 → T-09.06

## Definição de pronto

Uma branch, uma worktree e um volume mandados para a quarentena são restaurados idênticos; um `node_modules` é removido direto; um item alterado entre a análise e a execução é pulado com motivo.
