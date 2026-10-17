# ADR-0005 — Catálogo de locais e ecossistemas orientado a dados

- **Status:** Aceita
- **Data:** 2026-10-16
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-04.05, T-06.01, T-06.04, T-06.05, T-02.03

## Contexto

Os caminhos de caches, de discos virtuais e principalmente de ferramentas de IA mudam com frequência (novas versões, novos agentes). Caminhos fixos no código exigiriam uma nova versão do app para cada mudança, e testar com a máquina falsa exige poder redirecioná-los.

## Decisão

Locais conhecidos, marcadores de projeto, pastas de build, regras por ecossistema (risco, comando oficial de limpeza, como recriar) e locais de agentes ficam num **catálogo declarativo** distribuído com o app e extensível pelo usuário. Os coletores leem o catálogo; regras novas do mesmo formato não exigem código novo.

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **Catálogo declarativo** | Atualizável sem recompilar; redirecionável em testes; reforça I4 (só age em locais conhecidos) | Precisa de validação do formato |
| Caminhos no código | Simples | Fica desatualizado; difícil de testar |

## Consequências

**O que passa a ser obrigatório**
- Nenhum caminho de terceiros fixo no código dos coletores.
- O catálogo é validado ao carregar; entrada inválida é ignorada com aviso, nunca derruba a análise.
- Em builds de produção, o catálogo do usuário só **acrescenta** entradas; não pode remover proteções nem apontar para fora das pastas raiz e dos locais permitidos.
