# F10 — Automação: bandeja, agendamento e alertas

**Fase:** MVP · **Depende de:** F01, F02, F09

## Objetivo

Fazer o app trabalhar sozinho: ficar na bandeja, analisar periodicamente, limpar o que é 🟢 pelas regras e avisar quando vale a pena olhar.

## Fluxo do processo

1. O app inicia com o sistema (opcional) e fica na bandeja.
2. No horário agendado, roda uma análise em segundo plano.
3. Se a limpeza automática estiver ligada, executa apenas os itens 🟢 que passam nas regras (I8).
4. Se algum limite for ultrapassado, mostra uma notificação.
5. O ícone da bandeja reflete o estado (normal, lixo acima do limite, analisando).

## Regras-chave

- A automação nunca toca em 🟡, 🔴 ou ⛔.
- A automação nunca para Docker ou WSL nem fecha programas.
- Em bateria, a análise agendada é adiada.
- Toda ação automática é registrada com origem "automática".

## Ordem das tarefas

T-10.01 → T-10.02 → T-10.03 → T-10.04

## Definição de pronto

Com o app fechado na bandeja, a análise diária roda, limpa os caches 🟢 e avisa quando o recuperável passa do limite, sem nenhuma interação.
