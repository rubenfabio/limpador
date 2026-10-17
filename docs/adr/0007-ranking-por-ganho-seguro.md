# ADR-0007 — Ordenar o ranking por ganho seguro

- **Status:** Aceita
- **Data:** 2026-10-16
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-01.03, T-08.02

## Contexto

Ordenar só por tamanho coloca no topo itens grandes e arriscados (volumes, modelos em uso). O usuário quer ver primeiro o que libera mais espaço com menos chance de problema.

## Decisão

`ganho seguro = tamanho liberável × peso do risco × peso da atividade`, com pesos 🟢 1,0 · 🟡 0,6 · 🔴 0,2 · ⛔ 0 e atividade 1,0 (sem projeto ou inativo) · 0,3 (ativo). Ordenações por tamanho e por idade ficam disponíveis como alternativas. Itens de organização (tamanho 0) têm lista própria.

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **Ganho seguro** | O topo é acionável com confiança | Pesos são uma escolha arbitrária inicial |
| Só tamanho | Simples | Incentiva apagar o que é arriscado |
| Abas com várias visões, sem principal | Flexível | Sem resposta clara para "o que eu limpo primeiro?" |

## Consequências

- Os pesos são constantes documentadas no núcleo; torná-los configuráveis exige um ADR novo.
- Usar **tamanho liberável** (e não aparente) evita prometer espaço que não volta (hardlinks, camadas compartilhadas).
