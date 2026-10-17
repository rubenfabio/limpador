# ADR-0012 — Máquina falsa para testes e sandbox só em builds de teste

- **Status:** Aceita
- **Data:** 2026-10-17
- **Decisores:** Dono do produto
- **Tarefas relacionadas:** T-00.02, T-00.04

## Contexto

Os testes de integração e o e2e do app real precisam exercitar limpezas **reais** sem nunca tocar a máquina do desenvolvedor nem a do CI. Ao mesmo tempo, um "modo de teste" que redireciona caminhos seria perigoso se existisse no app distribuído.

## Decisão

- Um **gerador de cenários** monta, numa pasta temporária, uma máquina completa: projetos com datas controladas, repositórios git **reais**, caches, Downloads, locais de agentes e casos do Windows (caminho longo, junção, arquivo travado).
- **Docker, WSL e a API do GitHub são dublês** que respondem como os reais e registram os comandos recebidos.
- O núcleo e o app aceitam uma **configuração de sandbox** (catálogo, pasta de dados, dublês, relógio) **somente em builds de teste**, garantido em tempo de compilação.

## Alternativas consideradas

| Alternativa | Prós | Contras |
|---|---|---|
| **Máquina falsa + sandbox só em teste** | Testes realistas e seguros; build de produção sem esse vetor | Gerador de cenários é código a manter |
| Testar contra a máquina real | Nada a construir | Inaceitável: risco de apagar dados reais |
| Só dublês de sistema de arquivos | Rápido | Não prova o comportamento real do git nem do Windows |
| Docker real no CI | Fidelidade | Lento e instável no CI do Windows; fica opcional, sob variável explícita |

## Consequências

**O que passa a ser obrigatório**
- Nenhum teste usa caminhos fora da pasta temporária.
- Um teste verifica que o build de produção recusa a configuração de sandbox.
- Cada coletor novo acrescenta seus cenários ao gerador.
