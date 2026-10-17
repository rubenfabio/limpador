# ADR-0001 — Registrar decisões de arquitetura com ADRs

- **Status:** Aceita
- **Data:** 2026-10-17
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** todas

## Contexto

O projeto é construído em grande parte por agentes de IA, em sessões diferentes, sem memória compartilhada. Uma decisão que vive só numa conversa se perde, e o próximo agente pode desfazê-la ou reabrir a discussão.

## Decisão

Toda decisão de arquitetura ou técnica relevante é registrada como um ADR em `docs/adr/`, seguindo `docs/adr/000-modelo.md`, numerado em sequência (`NNNN-titulo-em-kebab-case.md`) e listado em `docs/adr/README.md`.

**É relevante** uma decisão que: escolhe tecnologia, biblioteca ou formato; define uma fronteira entre módulos; afeta segurança ou os invariantes I1–I10; é difícil de reverter; ou tem alternativas razoáveis que alguém poderia propor de novo.

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **ADRs em arquivos no repositório** | Versionados com o código, lidos por qualquer agente, revisáveis em PR | Exige disciplina para escrever |
| Seção "Decisões técnicas" no AGENTS.md | Simples | Cresce sem estrutura, sem histórico nem alternativas |
| Ferramenta externa (Notion, wiki) | Boa para leitura humana | Agentes não leem; fica fora de sincronia com o código |

## Consequências

**Positivas**
- Agentes e pessoas entendem o porquê das decisões e não as reabrem sem motivo.
- O histórico de mudanças fica explícito (ADRs substituídos não são apagados).

**Negativas / custos aceitos**
- Um pequeno custo de escrita a cada decisão.

**O que passa a ser obrigatório**
- Um agente que precise tomar uma decisão relevante cria o ADR com status **Proposta**, apresenta as opções ao dono do produto (com uma recomendada) e só implementa depois que ele mudar para **Aceita**.
- ADRs aceitos não são editados no conteúdo: para mudar uma decisão, cria-se um ADR novo, e o antigo passa a "Substituída por ADR-XXXX".
- O AGENTS.md aponta para os ADRs; ele não repete o conteúdo deles.
