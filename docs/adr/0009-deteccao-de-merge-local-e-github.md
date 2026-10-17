# ADR-0009 — Detectar merge combinando sinais locais e o GitHub

- **Status:** Aceita
- **Data:** 2026-10-16
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-05.02, T-05.03, T-05.05

## Contexto

Verificar só se uma branch é ancestral da padrão não detecta squash merge nem rebase merge, que são o padrão em PRs. Errar para o lado de "mergeada" pode apagar trabalho; errar para o outro lado deixa o lixo onde está.

## Decisão

- Combinar sinais locais (ancestralidade, upstream apagado, conteúdo equivalente, commits únicos, espelho do remoto) com o estado do PR no GitHub, quando houver autenticação **já existente** na máquina.
- Um PR mergeado só vale se a ponta local for igual ao último commit do PR ou ancestral dele.
- Quando as fontes divergem, vale o veredito **mais conservador**.
- O app não pede token no MVP; sem autenticação, funciona com sinais locais e mostra "verificação limitada".
- A análise não faz fetch por padrão (I1).

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **Local + GitHub** | Pega squash e rebase com alta confiança | Depende de rede e autenticação para o melhor resultado |
| Só local | Offline e para qualquer host | Mais falsos negativos e mais 🟡 |
| Só GitHub | Simples | Inútil sem rede e para outros hosts |

## Consequências

- Outros hosts (GitLab, Bitbucket, GitHub Enterprise) entram depois, com um ADR novo.
- Os vereditos são **código crítico** (ADR-0011).
