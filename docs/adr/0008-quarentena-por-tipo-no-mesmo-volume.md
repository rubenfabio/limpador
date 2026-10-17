# ADR-0008 — Quarentena por tipo de item, no mesmo volume, com prazo

- **Status:** Aceita
- **Data:** 2026-10-16
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-09.04, T-09.05, T-09.06, T-05.04, T-05.05, T-05.06, T-03.03, T-03.04

## Contexto

Itens com dado único (branches, worktrees, stashes, volumes, arquivos pessoais) precisam poder voltar. A lixeira do Windows não sabe guardar uma branch nem um volume Docker, e copiar gigabytes entre discos é lento.

## Decisão

- Uma quarentena própria, com **estratégia por tipo**: pastas são movidas no mesmo volume; branches e stashes guardam referências protegidas da coleta de lixo do git; volumes são exportados para arquivo; containers ficam como snapshot da camada gravável e da configuração.
- **Integridade verificada antes** de remover o original.
- Prazo padrão de **14 dias**, fixado na entrada. O espaço só volta na purga, e a interface diz isso.
- Restaurar **nunca sobrescreve**: em caso de conflito, oferece outro nome ou cancela.

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **Quarentena própria por tipo** | Restaura qualquer tipo; rápida no mesmo volume | Mais código para manter, um por tipo |
| Lixeira do sistema | Familiar | Não cobre git nem Docker; esvaziada fora do controle do app |
| Backup compactado de tudo | Uniforme | Lento e ocupa espaço extra justamente quando o disco está cheio |

## Consequências

**O que passa a ser obrigatório**
- Todo tipo "conteúdo do usuário" implementa entrada, verificação, restauração e purga (ADR-0004).
- Entrada, restauração e purga são **código crítico** (ADR-0011).
