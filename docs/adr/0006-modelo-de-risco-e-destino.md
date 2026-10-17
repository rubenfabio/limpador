# ADR-0006 — Modelo de risco em quatro níveis com destino derivado

- **Status:** Aceita
- **Data:** 2026-10-16
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-01.01, T-01.02, T-09.04, T-10.03

## Contexto

O app precisa decidir de forma consistente o que pode ser apagado direto, o que precisa de rede de segurança e o que nunca deve ser tocado. A decisão do produto foi: automação só para itens seguros, e a quarentena depende do risco.

## Decisão

- Quatro níveis: 🟢 Seguro · 🟡 Moderado · 🔴 Cuidado · ⛔ Bloqueado; "Protegido" fica fora do ranking.
- Cada item tem uma **natureza**: recriável ou conteúdo do usuário.
- O risco final é o **mais severo** entre os sinais.
- O **destino é derivado**, nunca escolhido por quem chama: conteúdo do usuário ou 🔴 → quarentena; senão → exclusão direta; ⛔ → nenhuma ação.
- **Elegível para automação** = 🟢 + recriável + sem efeitos colaterais + não protegido.

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **4 níveis + natureza + destino derivado** | Regras simples de auditar; o destino não pode ser burlado | Alguns casos de fronteira precisam de regra explícita |
| Pontuação contínua de risco | Mais nuance | Difícil de explicar ao usuário e de testar |
| Tudo vai para a quarentena | Máxima segurança | Não libera espaço na hora; caches não precisam disso |

## Consequências

**O que passa a ser obrigatório**
- O cálculo de risco, destino e elegibilidade fica num único lugar do núcleo e é **código crítico** (mutação ≥ 90%, ADR-0011).
- Nenhuma casca (UI, CLI, MCP) pode passar um destino diferente do derivado.
