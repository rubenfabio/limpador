# F08 — Interface: ranking e exploração

**Fase:** MVP · **Depende de:** F01 (e dos coletores para ter dados reais)

## Objetivo

Mostrar de forma clara quanto pode ser recuperado, o que é seguro e por quê, e deixar o usuário escolher o que limpar.

## Telas e fluxo

1. **Painel**: espaço recuperável total por risco, disco livre, top itens por ganho seguro, botão Analisar.
2. **Ranking**: lista completa por ganho seguro, com filtros.
3. **Por projeto / por categoria**: as mesmas informações agrupadas.
4. **Detalhe do item**: motivo, risco, como recriar, conteúdo e projeto.
5. **Seleção**: o usuário marca itens e vê o resumo do que será feito.
6. Daí segue para a confirmação e execução (F09).

## Regras-chave

- Nenhum item aparece sem motivo legível.
- Itens ⛔ aparecem, mas não podem ser selecionados, e mostram o porquê.
- Todos os textos vêm de chaves de tradução (pt-BR).
- A interface nunca trava durante a análise: progresso visível e cancelável.

## Ordem das tarefas

T-08.07 → T-08.06 → T-08.01 → T-08.02 → T-08.03 → T-08.04 → T-08.05

## Definição de pronto

O usuário entende em menos de 10 segundos quanto pode recuperar com segurança e consegue selecionar e revisar uma limpeza sem ler documentação.
