# F00 — Qualidade e testes

**Fase:** MVP (vem antes de todas) · **Depende de:** nada

## Objetivo

Montar a fundação de testes **antes** do primeiro código de negócio, para que toda tarefa seguinte já nasça com testes primeiro, mutação e e2e. Estratégia completa em `docs/estrategia-de-testes.md`.

## Escopo

**Entra:** esqueleto do projeto com testes unitários rodando, máquina falsa (gerador de cenários + dublês), e2e de interface, e2e do app real, mutação com metas, verificações locais e CI, rastreabilidade CA → teste.

**Não entra:** os testes de cada feature, que são escritos dentro das tarefas de cada uma.

## Fluxo do processo

1. Criar o esqueleto do app com um teste unitário de exemplo em cada lado (núcleo e interface).
2. Construir a máquina falsa, de forma incremental: começa com o que F01 e F02 precisam e cresce a cada coletor.
3. Ligar o e2e de interface com núcleo simulado.
4. Ligar o e2e do app real com uma jornada mínima ("abre e mostra o painel").
5. Configurar a mutação com as metas por criticidade.
6. Configurar as verificações antes do commit e o CI.
7. Ligar o verificador de rastreabilidade.

## Regras-chave

- Nenhum teste toca a máquina real (I1–I10).
- O modo sandbox existe **só** em builds de teste.
- Meta de mutação não atingida bloqueia o merge.
- A máquina falsa é código de primeira classe: tem os próprios testes e é reutilizada por todas as camadas.

## Ordem das tarefas

T-00.01 → T-00.02 → T-00.03 → T-00.04 → T-00.05 → T-00.06 → T-00.07

As tarefas T-00.02 e T-00.04 continuam crescendo: cada coletor novo acrescenta cenários à máquina falsa, e cada jornada nova acrescenta um e2e.

## Definição de pronto

Um push no repositório roda, no Windows, unitários, integração, os dois níveis de e2e, a mutação incremental com metas e a rastreabilidade, e falha se qualquer um falhar.
