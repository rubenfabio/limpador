# F11 — Histórico e auditoria

**Fase:** MVP · **Depende de:** F01, F09

## Objetivo

Registrar tudo o que o app fez, de forma auditável, e mostrar a evolução do lixo e do espaço recuperado ao longo do tempo.

## Fluxo do processo

1. Cada ação (limpeza, quarentena, restauração, purga, compactação, falha) gera uma entrada no log.
2. Cada análise gera um snapshot com os totais.
3. A tela de histórico mostra o log com filtros e os gráficos de evolução.
4. Entradas antigas expiram conforme a retenção.

## Regras-chave

- O log é a fonte de verdade para "o que aconteceu com o meu X?".
- Entradas de itens ainda na quarentena nunca expiram antes da purga.

## Ordem das tarefas

T-11.01 → T-11.02 → T-11.03

## Definição de pronto

O usuário encontra no histórico quando e por que qualquer item foi removido, e vê num gráfico quanto espaço já recuperou no total.
