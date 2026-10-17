# F01 — Motor de análise

**Fase:** MVP · **Depende de:** nada (base de tudo)

## Objetivo

Definir o modelo de domínio (item limpável, risco, projeto) e o motor que executa os coletores, consolida os resultados e produz o ranking por ganho seguro.

## Escopo

**Entra:** modelo de item, classificação de risco, cálculo de ganho seguro, contrato que todo coletor segue, orquestração da análise (paralelismo, progresso, cancelamento, falhas isoladas), cálculo de atividade de projeto.

**Não entra:** coletores específicos (F03–F07), interface (F08), remoção (F09).

## Fluxo do processo

1. O usuário (ou o agendamento) pede uma análise.
2. O motor carrega as configurações (pastas raiz, proteções, limiares).
3. O motor pergunta a cada coletor registrado se o ecossistema dele existe na máquina (`detectar`).
4. Os coletores disponíveis rodam em paralelo (`analisar`) e reportam progresso.
5. Cada item recebe projeto associado, atividade, risco final (aplicando proteções) e ganho seguro.
6. O motor consolida o resultado: totais por risco, por categoria, por projeto e o ranking.
7. O resultado é entregue à interface e gravado como snapshot no histórico (F11).

## Regras-chave

- A análise é **somente leitura** (I1).
- A falha de um coletor não derruba a análise: o resultado mostra o coletor com erro e o motivo.
- O resultado é sempre reproduzível: mesma máquina e mesmo estado geram o mesmo ranking.

## Ordem das tarefas

T-01.01 → T-01.02 → T-01.06 → T-01.03 → T-01.04 → T-01.05

## Definição de pronto

Um coletor de teste (fictício) registrado no motor gera itens que saem ranqueados corretamente, com totais por risco, categoria e projeto, e com progresso e cancelamento funcionando.
