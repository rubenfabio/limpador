# ADR-0014 — Textos por chave de tradução, inclusive os gerados pelo núcleo

- **Status:** Aceita
- **Data:** 2026-10-16
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-08.07, T-01.01

## Contexto

A interface começa em pt-BR, e o inglês entra quando o código for aberto. Grande parte do texto visível (motivos, alertas, "como recriar") é produzida pelos coletores no núcleo, não pela interface.

## Decisão

- Todo texto visível usa **chave de tradução**.
- O núcleo **nunca produz frases prontas**: emite chave + parâmetros (ex.: `reason.image_unused { tag, days, size }`), e a casca (interface, CLI, MCP) monta o texto no idioma do usuário.
- Formatação de números, tamanhos e datas acontece só na casca, conforme o idioma.

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **Chave + parâmetros desde o núcleo** | Traduzir não exige mexer no núcleo; CLI e MCP reaproveitam | Um pouco mais verboso |
| Núcleo gera texto em pt-BR | Simples agora | Retrabalho grande ao traduzir |

## Consequências

- Um teste verifica que não há texto visível fora dos arquivos de tradução (T-08.07).
