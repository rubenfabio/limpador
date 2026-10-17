# F02 — Configuração inicial e descoberta de projetos

**Fase:** MVP · **Depende de:** F01

## Objetivo

Saber **onde** procurar e **o que proteger**: pastas raiz, identificação de projetos, proteções e limiares configuráveis.

## Escopo

**Entra:** primeira execução (onboarding), gestão de pastas raiz e exclusões, reconhecimento de projetos, proteções (projetos fixados, caminhos protegidos, branches protegidas), preferências e limiares.

**Não entra:** agendamento (F10), que só consome as preferências daqui.

## Fluxo do processo

1. Na primeira execução, o app sugere pastas raiz candidatas e o usuário confirma.
2. O app pergunta se a limpeza automática de itens 🟢 deve ficar ligada.
3. O app roda a primeira análise.
4. A qualquer momento, o usuário ajusta pastas, exclusões, proteções e limiares nas configurações.

## Regras-chave

- Nada é analisado fora das pastas raiz e dos locais do catálogo (I4).
- Proteções vencem qualquer outra regra (I10).
- Toda configuração tem valor padrão sensato e pode ser restaurada.

## Ordem das tarefas

T-02.05 → T-02.01 → T-02.02 → T-02.03 → T-02.04

## Definição de pronto

Um usuário novo abre o app, confirma as pastas sugeridas e vê seus projetos reconhecidos; um projeto fixado nunca aparece como inativo.
